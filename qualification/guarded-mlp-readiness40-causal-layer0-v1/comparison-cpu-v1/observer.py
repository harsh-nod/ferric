"""Observe genuine layer-zero calls; never replace a tensor or recompute a stage."""
import hashlib
import json
import sys

SHAPES = {
    'embedding': (1,1,4096), 'input-norm-input': (1,1,4096), 'input-norm': (1,1,4096),
    'q-input': (1,1,4096), 'k-input': (1,1,4096), 'v-input': (1,1,4096),
    'q-projection': (1,1,4096), 'k-projection': (1,1,1024), 'v-projection': (1,1,1024),
    'q-norm-input': (1,1,32,128), 'q-norm': (1,1,32,128),
    'k-norm-input': (1,1,8,128), 'k-norm': (1,1,8,128),
    'rotary-cos': (1,1,128), 'rotary-sin': (1,1,128),
    'rotary-q': (1,32,1,128), 'rotary-k': (1,8,1,128),
    'attention-output': (1,1,4096), 'o-projection': (1,1,4096),
    'first-residual': (1,1,4096), 'post-norm': (1,1,4096),
    'gate-input': (1,1,4096), 'up-input': (1,1,4096), 'gate': (1,1,12288),
    'up': (1,1,12288), 'silu-input': (1,1,12288), 'silu': (1,1,12288),
    'product': (1,1,12288), 'down-projection': (1,1,4096), 'mlp-output': (1,1,4096),
    'layer0-hidden': (1,1,4096),
}
JOINS = (('embedding','input-norm-input'), ('input-norm','q-input'),
    ('input-norm','k-input'), ('input-norm','v-input'), ('q-projection','q-norm-input'),
    ('k-projection','k-norm-input'), ('post-norm','gate-input'), ('post-norm','up-input'),
    ('gate','silu-input'), ('down-projection','mlp-output'))

def require(ok, message):
    if not ok:
        raise ValueError(message)

def pin(raw):
    return dict(bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())

def shapes(position):
    require(type(position) is int and 0 <= position < 6, 'exact causal position0..5')
    return dict(SHAPES, **{'cache-key': (1,8,position+1,128), 'cache-value': (1,8,position+1,128)})

def cache_rows(raw, position, rank):
    """Byte permutation only: framework head/token/channel to native token/head/channel."""
    require(type(position) is int and 0 <= position < 6 and type(rank) is int and rank in (0,1)
            and type(raw) is bytes and len(raw) == 8 * (position + 1) * 256, 'closed cache dimensions')
    return b''.join(raw[(head * (position+1) + token)*256:(head * (position+1) + token+1)*256]
                    for token in range(position+1) for head in range(rank*4, rank*4+4))

def validate_values(values, position, previous=None):
    expected = shapes(position)
    require(type(values) is dict and set(values) == set(expected), 'complete actual stage roster')
    for name, shape in expected.items():
        size = 2
        for n in shape:
            size *= n
        raw = values[name]
        require(type(raw) is bytes and len(raw) == size, 'stage BF16 extent')
        require(all(int.from_bytes(raw[i:i+2], 'little') & 0x7f80 != 0x7f80 for i in range(0,size,2)),
                'finite actual BF16 stage')
    require(all(values[a] == values[b] for a,b in JOINS), 'actual producer-consumer byte joins')
    for cache, current in [('cache-key','rotary-k'),('cache-value','v-projection')]:
        last = b''.join(values[cache][(head*(position+1)+position)*256:(head*(position+1)+position+1)*256]
                        for head in range(8))
        require(last == values[current], 'actual current K/V appended to own cache')
        if position:
            require(type(previous) is dict, 'genuine preceding capture required')
            for rank in (0,1):
                require(cache_rows(values[cache],position,rank).startswith(cache_rows(previous[cache],position-1,rank)),
                        'completed causal cache changed')
    require((previous is None) == (position == 0), 'exact capture ancestry')


class Observe:
    def __init__(self, model, torch, raw, position, bounded):
        self.model, self.torch, self.raw, self.position, self.bounded = model, torch, raw, position, bounded
        self.shapes = shapes(position)
        self.values, self.handles = {}, []
        self.module = sys.modules[model.__class__.__module__]
        self.layer = model.model.layers[0]
        self.original = self.module.apply_rotary_pos_emb
        self.wrapper = self.rotary
        self.active, self.calls, self.selected, self.installed = False, 0, 0, False

    def add(self, name, tensor):
        require(name in self.shapes and name not in self.values, 'one actual stage invocation')
        self.values[name] = self.raw(tensor, self.shapes[name])
        self.bounded()

    def pre(self, name):
        def hook(_owner, arguments):
            require(type(arguments) is tuple and len(arguments) >= 1, 'actual positional module input')
            self.add(name, arguments[0])
        return hook

    def post(self, name):
        def hook(_owner, _arguments, output):
            self.add(name, output)
        return hook

    def enter(self, _owner, _arguments):
        require(not self.active, 'non-nested layer0 attention')
        self.active = True

    def leave(self, _owner, _arguments, _output):
        require(self.active, 'layer0 attention entered')
        self.active = False

    def rotary(self, *args, **kwargs):
        result = self.original(*args, **kwargs)
        self.calls += 1
        if self.active:
            require(self.selected == 0 and type(result) is tuple and len(result) == 2, 'one actual rotary pair')
            self.add('rotary-q', result[0]); self.add('rotary-k', result[1])
            self.selected += 1
        return result

    def attach(self):
        require(not self.installed and self.layer.self_attn.forward.__func__.__globals__ is self.module.__dict__
                and self.model.config._attn_implementation == 'sdpa'
                and isinstance(self.layer.mlp.act_fn, self.torch.nn.Module), 'same qualified actual SDPA/SiLU route')
        l = self.layer
        bindings = [(self.model.model.embed_tokens,None,'embedding'),
            (l.input_layernorm,'input-norm-input','input-norm'),
            (l.self_attn.q_proj,'q-input','q-projection'), (l.self_attn.k_proj,'k-input','k-projection'),
            (l.self_attn.v_proj,'v-input','v-projection'), (l.self_attn.q_norm,'q-norm-input','q-norm'),
            (l.self_attn.k_norm,'k-norm-input','k-norm'), (l.self_attn.o_proj,'attention-output','o-projection'),
            (l.post_attention_layernorm,'first-residual','post-norm'),
            (l.mlp.gate_proj,'gate-input','gate'), (l.mlp.up_proj,'up-input','up'),
            (l.mlp.act_fn,'silu-input','silu'), (l.mlp.down_proj,'product','down-projection'),
            (l.mlp,None,'mlp-output')]
        def rotation(_owner, _arguments, output):
            require(type(output) is tuple and len(output)==2, 'actual rotary cos/sin')
            self.add('rotary-cos',output[0]); self.add('rotary-sin',output[1])
        def layer(_owner, _arguments, output):
            require(type(output) is tuple and len(output)>=1, 'actual decoder output')
            self.add('layer0-hidden',output[0])
        try:
            for owner,before,after in bindings:
                if before: self.handles.append(owner.register_forward_pre_hook(self.pre(before)))
                if after: self.handles.append(owner.register_forward_hook(self.post(after)))
            self.handles.append(self.model.model.rotary_emb.register_forward_hook(rotation))
            self.handles.append(l.self_attn.register_forward_pre_hook(self.enter))
            self.handles.append(l.self_attn.register_forward_hook(self.leave))
            self.handles.append(l.register_forward_hook(layer))
            require(self.module.apply_rotary_pos_emb is self.original, 'original rotary still installed')
            self.module.apply_rotary_pos_emb = self.wrapper
            self.installed = True
        except BaseException:
            self.close()
            raise

    def finish(self, cache, previous):
        require(self.installed and not self.active and self.calls==36 and self.selected==1,
                'one layer0 observer inside genuine full36 forward')
        require(cache.get_seq_length()==self.position+1 and len(cache.key_cache)==len(cache.value_cache)==36,
                'actual full-model DynamicCache history')
        self.add('cache-key',cache.key_cache[0]); self.add('cache-value',cache.value_cache[0])
        validate_values(self.values,self.position,previous)
        return self.values

    def close(self):
        changed = False
        if self.installed:
            changed = self.module.apply_rotary_pos_emb is not self.wrapper
            self.module.apply_rotary_pos_emb = self.original
            self.installed = False
        failures = []
        for handle in reversed(self.handles):
            try:
                handle.remove()
            except BaseException as error:
                failures.append(error)
        self.handles.clear()
        require(not changed, 'observer callable changed before restoration')
        require(not failures, 'observer hook removal failed')


def encode(captures, tokens, ordinal):
    require(type(ordinal) is int and ordinal in (1,2) and len(captures)==6 and len(tokens)==40,
            'two independent six-prefix diagnostic passes inside40')
    payload, rows = bytearray(), []
    for position, values in enumerate(captures):
        validate_values(values,position,captures[position-1] if position else None)
        parts = []
        for name, shape in shapes(position).items():
            raw=values[name]
            parts.append(dict(name=name, shape=list(shape), dtype='bfloat16', offset=len(payload), **pin(raw)))
            payload.extend(raw)
        rows.append(dict(position=position, input_token=tokens[position], parts=parts))
    require(len(payload)<=4<<20, 'bounded framework sidecar')
    head=json.dumps(dict(schema='FerricReadiness40CausalFrameworkLayerZeroV1', ordinal=ordinal,
        full_model_forward_calls=40, captures=rows, payload=pin(payload), native_intermediates_used=False,
        numerical_acceptance=False, performance_claim=False),sort_keys=True,separators=(',',':')).encode()
    require(len(head)<=128<<10, 'bounded sidecar metadata')
    return b'FREF061\0'+len(head).to_bytes(4,'little')+len(payload).to_bytes(4,'little')+head+payload
