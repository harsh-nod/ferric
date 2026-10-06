python
import gdb
import json
import os
import struct

ELF_SHA = "dbf5ea87824b4ab2be1e481cf21595f5809fb95bd309552e888a32f8089a8425"
ELF = "/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/guarded-mlp-interleaved-native-cpu-v228-v1/target/debug/deps/fe2o3_kfd-11448a9880d552d9"
PREFIX = "fe2o3_kfd::engineering_gfx950::peer::combined_mlp_state_v1::"
REGION = PREFIX + "paired::profiles::region"
PREPARE = PREFIX + "paired::profiles::prepare"
FIXTURE = PREFIX + "tests::native::paired::interleaved::interleaved_exercise"
ALLOCATE = "<" + PREFIX + "paired::retained::RetainedPair>::allocate"
MODE = os.environ["FERRIC_DEBUGGER_MODE_V1"]
assert MODE in ("inventory", "native")
# Synchronous target control prevents ROCgdb's per-inferior AMDGPU attach.
# The program still uses its ordinary KFD queues; only host state is inspected.
gdb.execute("maintenance set target-async off")
target_async_setting = gdb.execute("maintenance show target-async", to_string=True)
assert "off" in target_async_setting.lower()
for command in ("set pagination off", "set confirm off", "set print thread-events off",
                "set print frame-arguments none",
                "set language c", "set startup-with-shell off", "set disable-randomization off"):
    gdb.execute(command)

def address(name):
    return int(gdb.parse_and_eval("(unsigned long long)&'" + name + "'"))

def reg(name):
    return int(gdb.parse_and_eval("$" + name))

def read(address, fmt):
    return struct.unpack(fmt, bytes(gdb.selected_inferior().read_memory(
        address, struct.calcsize(fmt))))[0]

def words(address):
    return [read(address + 8 * i, "<Q") for i in range(4)]

exit_codes = []
gdb.events.exited.connect(lambda event: exit_codes.append(getattr(event, "exit_code", None)))
gdb.execute("starti", to_string=True)
inferior_pid = gdb.selected_inferior().pid
assert inferior_pid > 1 and os.readlink("/proc/" + str(inferior_pid) + "/exe") == ELF
with open("/proc/" + str(inferior_pid) + "/stat", "r") as stream:
    fields = stream.read(8192).rsplit(") ", 1)[1].split()
assert int(fields[1]) == os.getpid()
assert all(address(name) > 0 for name in (REGION, PREPARE, FIXTURE, ALLOCATE))
target_stack = gdb.execute("maintenance print target-stack", to_string=True)
assert "amd-dbgapi" not in target_stack.lower()
gdb.write("\nFERRIC_DEBUGGER_START_V1=" + json.dumps(dict(
    mode=MODE, debugger_pid=os.getpid(), inferior_pid=inferior_pid,
    inferior_pgid=int(fields[2]), inferior_sid=int(fields[3]),
    inferior_start_ticks=int(fields[19]), elf_sha256=ELF_SHA,
    host_only_debugger=True, target_async_setting=target_async_setting,
    target_stack=target_stack), sort_keys=True) + "\n")

chain = []
context = {}

class TokenStage(gdb.Breakpoint):
    def __init__(self, stage, symbol, offset):
        super().__init__("*'" + symbol + "'+" + hex(offset), internal=True)
        self.stage, self.symbol, self.offset, self.hits = stage, symbol, offset, 0

    def stop(self):
        # Python stop callbacks run before the CLI breakpoint condition. Filter
        # here without changing breakpoints or inferior state from the callback.
        if self.hits or (self.stage == "selection" and (reg("rbp") != 1 or reg("r14") != 1)):
            return False
        ordinal = ("selection", "transfer", "prepare").index(self.stage)
        if len(chain) != ordinal:
            return False
        self.hits += 1
        assert self.hits == 1
        assert len(chain) == ordinal
        thread = gdb.selected_thread().ptid
        value = dict(schema="ferric-original-common-root-chain-observation-v1",
                     stage=self.stage, ordinal=ordinal, hit=self.hits, elf_sha256=ELF_SHA,
                     site_offset=reg("pc") - address(self.symbol))
        assert value["site_offset"] == self.offset
        if self.stage == "selection":
            stack = reg("rsp")
            common = read(stack + 0x1a0, "<Q")
            actual = reg("rax") + 4 * reg("r12")
            assert read(stack + 0x1a8, "<Q") == 2
            rows = [[words(common + 128 * rank + 32 * index) for index in range(4)]
                    for rank in range(2)]
            context.update(thread=thread, stack=stack, common=common, rows=rows)
            value.update(rank=reg("rbp"), root_index=reg("r14"), stride_counter=reg("r12"),
                         common_len=2, common_rows=rows, selected_token=words(actual),
                         actual_delta=actual - common,
                         expected_delta=128 * reg("rbp") + 32 * (reg("r14") - 1))
        elif self.stage == "transfer":
            stack = reg("rsp")
            assert thread == context["thread"] and stack == context["stack"]
            context["inputs"] = stack + 0x22f0
            common = read(stack + 0x1a0, "<Q")
            rows = [[words(common + 128 * rank + 32 * index) for index in range(4)]
                    for rank in range(2)]
            value.update(same_thread=True, same_stack=True,
                         return_matches_inputs=reg("rax") == context["inputs"],
                         common_unchanged=common == context["common"] and rows == context["rows"],
                         payload_token=words(stack + 0x49c8), inputs_token=words(stack + 0x24d0))
        else:
            assert thread == context["thread"]
            value.update(same_thread=True, inputs_pointer_matches=reg("rcx") == context["inputs"],
                         inputs_token=words(reg("rcx") + 0x1e0))
        chain.append(value)
        prefixes = ("FERRIC_COMMON_SELECTION_V1=", "FERRIC_PAYLOAD_TRANSFER_V1=", "FERRIC_PREPARE_INPUT_V1=")
        gdb.write("\n" + prefixes[ordinal] + json.dumps(value, sort_keys=True) + "\n")
        return False

class RoleFailure(gdb.Breakpoint):
    def __init__(self):
        super().__init__("*'" + REGION + "'+0x136", internal=True)
        self.hits = 0

    def stop(self):
        if self.hits:
            return False
        self.hits += 1
        assert self.hits == 1
        assert len(chain) == 3 and gdb.selected_thread().ptid == context["thread"]
        stack, token, record = reg("rsp"), reg("r15"), reg("rsi")
        return_offset = read(stack + 0x78, "<Q") - address(PREPARE)
        roles = {0x10a: "root", 0x189: "combined",
                 0x3ae: "partial0", 0x636: "partial1",
                 0x6da: "residual0", 0x760: "residual1",
                 0x7cd: "output0", 0x84f: "output1"}
        role = roles.get(return_offset, "unknown")
        inputs, owners = read(stack + 0xc0, "<Q"), read(stack + 0xc8, "<Q")
        saved_rank, saved_byte_index = read(stack + 0x58, "<Q"), read(stack + 0x50, "<Q")
        pointers = dict(root=inputs + 0x20 + saved_rank * 0x1a0 + saved_byte_index,
                        combined=owners + saved_rank * 0x30, partial0=inputs + 0x340,
                        partial1=inputs + 0x360, residual0=inputs + 0x160,
                        residual1=inputs + 0x300, output0=inputs + 0x180, output1=inputs + 0x320)
        source, copied, recorded = words(token), words(stack + 0x20), words(record + 0x28)
        value = dict(schema="ferric-original-mlp-role-predicate-observation-v1",
                     hit=self.hits, elf_sha256=ELF_SHA,
                     region_pc_offset=reg("pc") - address(REGION),
                     prepare_return_offset=return_offset, role=role,
                     root_index=saved_byte_index // 32 if role == "root" else None,
                     saved_rank=saved_rank, saved_byte_index=saved_byte_index,
                     source_address_matches_role=token == pointers.get(role),
                     inputs_pointer_matches_transfer=inputs == context["inputs"],
                     source_token=source, copied_token=copied, record_token=recorded,
                     expected_owner=reg("r12"), expected_bytes=reg("r14"),
                     combined_u32=reg("ebp") & 0xffffffff, record_kind_u8=read(record + 0x50, "<B"),
                     mapping_u8=read(record + 0x20, "<B"),
                     validation_tag_u64=read(stack + 8, "<Q"),
                     group_id=read(reg("r13") + 0x108, "<Q"),
                     record_pointer_matches_return=read(stack + 0x10, "<Q") == record,
                     eax_u32=reg("eax") & 0xffffffff, ecx_u32=reg("ecx") & 0xffffffff,
                     eflags_u64=reg("eflags"))
        gdb.write("\nFERRIC_ROLE_PREDICATE_V1=" + json.dumps(value, sort_keys=True) + "\n")
        gdb.execute("backtrace 6")
        return False

stages = [TokenStage("selection", FIXTURE, 0x1391), TokenStage("transfer", FIXTURE, 0x17ef),
          TokenStage("prepare", ALLOCATE, 0x463)] if MODE == "native" else []
breakpoint = RoleFailure() if MODE == "native" else None
gdb.execute("continue")
if gdb.selected_inferior().pid:
    gdb.execute("kill", to_string=True)
    raise gdb.GdbError("inferior stopped before a natural exit")
gdb.write("\nFERRIC_DEBUGGER_EXIT_V1=" + json.dumps(dict(
    mode=MODE, exit_codes=exit_codes, hits=breakpoint.hits if breakpoint else 0),
    sort_keys=True) + "\n")
end
