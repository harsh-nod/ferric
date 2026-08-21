//! Directly verified SHA-256 computation for offline artifact identity.
//!
//! The executable bodies refine the closed schedule, compression, streaming,
//! padding, and digest functions below for messages shorter than 2^64 bits.
//! This is a computation theorem, not a collision-resistance, preimage-
//! resistance, signature, provenance, or artifact-authentication theorem.

use vstd::prelude::*;

verus! {

closed spec fn rotate_right_spec(value: u32, count: u32) -> u32
    recommends 0 < count < 32,
{
    (value >> count) | (value << ((32 - count) as u32))
}

#[verifier::bit_vector]
proof fn shifted_rotation_parts_are_disjoint(value: u32, count: u32)
    requires 0 < count < 32,
    ensures
        (value >> count) ^ (value << ((32 - count) as u32))
            == (value >> count) | (value << ((32 - count) as u32)),
{
}

fn rotate_right(value: u32, count: u32) -> (rotated: u32)
    requires 0 < count < 32,
    ensures rotated == rotate_right_spec(value, count),
{
    proof {
        shifted_rotation_parts_are_disjoint(value, count);
    }
    let complementary_count = 32 - count;
    (value >> count) ^ (value << complementary_count)
}

closed spec fn big_endian_word(block: Seq<u8>, index: int) -> u32
    recommends block.len() == 64, 0 <= index < 16,
{
    ((block[index * 4] as u32) << 24)
        | ((block[index * 4 + 1] as u32) << 16)
        | ((block[index * 4 + 2] as u32) << 8)
        | (block[index * 4 + 3] as u32)
}

closed spec fn small_sigma0(value: u32) -> u32 {
    rotate_right_spec(value, 7) ^ rotate_right_spec(value, 18) ^ (value >> 3)
}

closed spec fn small_sigma1(value: u32) -> u32 {
    rotate_right_spec(value, 17) ^ rotate_right_spec(value, 19) ^ (value >> 10)
}

closed spec fn schedule_prefix(block: Seq<u8>, count: nat) -> Seq<u32>
    recommends block.len() == 64, count <= 64,
    decreases count,
{
    if count == 0 {
        Seq::empty()
    } else {
        let previous = schedule_prefix(block, (count - 1) as nat);
        let index = count - 1;
        if index < 16 {
            previous.push(big_endian_word(block, index as int))
        } else {
            previous.push(
                previous[(index - 16) as int]
                    .wrapping_add(small_sigma0(previous[(index - 15) as int]))
                    .wrapping_add(previous[(index - 7) as int])
                    .wrapping_add(small_sigma1(previous[(index - 2) as int])),
            )
        }
    }
}

closed spec fn initial_working_state(
    state: Seq<u32>,
) -> (u32, u32, u32, u32, u32, u32, u32, u32)
    recommends state.len() == 8,
{
    (state[0], state[1], state[2], state[3], state[4], state[5], state[6], state[7])
}

closed spec fn round(
    state: (u32, u32, u32, u32, u32, u32, u32, u32),
    schedule_word: u32,
    constant: u32,
) -> (u32, u32, u32, u32, u32, u32, u32, u32) {
    let sum1 = rotate_right_spec(state.4, 6)
        ^ rotate_right_spec(state.4, 11)
        ^ rotate_right_spec(state.4, 25);
    let choice = (state.4 & state.5) ^ (!state.4 & state.6);
    let temporary1 = state.7
        .wrapping_add(sum1)
        .wrapping_add(choice)
        .wrapping_add(constant)
        .wrapping_add(schedule_word);
    let sum0 = rotate_right_spec(state.0, 2)
        ^ rotate_right_spec(state.0, 13)
        ^ rotate_right_spec(state.0, 22);
    let majority = (state.0 & state.1) ^ (state.0 & state.2) ^ (state.1 & state.2);
    let temporary2 = sum0.wrapping_add(majority);
    (
        temporary1.wrapping_add(temporary2),
        state.0,
        state.1,
        state.2,
        state.3.wrapping_add(temporary1),
        state.4,
        state.5,
        state.6,
    )
}

closed spec fn rounds(
    state: (u32, u32, u32, u32, u32, u32, u32, u32),
    schedule: Seq<u32>,
    count: nat,
) -> (u32, u32, u32, u32, u32, u32, u32, u32)
    recommends schedule.len() == 64, count <= 64,
    decreases count,
{
    if count == 0 {
        state
    } else {
        let previous = rounds(state, schedule, (count - 1) as nat);
        round(
            previous,
            schedule[(count - 1) as int],
            ROUND_CONSTANTS@[(count - 1) as int],
        )
    }
}

closed spec fn compress_block(state: Seq<u32>, block: Seq<u8>) -> Seq<u32>
    recommends state.len() == 8, block.len() == 64,
{
    let working = rounds(initial_working_state(state), schedule_prefix(block, 64), 64);
    seq![
        state[0].wrapping_add(working.0),
        state[1].wrapping_add(working.1),
        state[2].wrapping_add(working.2),
        state[3].wrapping_add(working.3),
        state[4].wrapping_add(working.4),
        state[5].wrapping_add(working.5),
        state[6].wrapping_add(working.6),
        state[7].wrapping_add(working.7),
    ]
}

closed spec fn absorb(
    core: (Seq<u32>, Seq<u8>),
    input: Seq<u8>,
) -> (Seq<u32>, Seq<u8>)
    recommends core.0.len() == 8, core.1.len() < 64,
    decreases input.len(),
{
    if input.len() == 0 {
        core
    } else {
        let filled = core.1.push(input[0]);
        let next = if filled.len() == 64 {
            (compress_block(core.0, filled), Seq::empty())
        } else {
            (core.0, filled)
        };
        absorb(next, input.subrange(1, input.len() as int))
    }
}

pub(super) closed spec fn update_view(
    view: ((Seq<u32>, Seq<u8>), nat),
    input: Seq<u8>,
) -> ((Seq<u32>, Seq<u8>), nat)
    recommends view.0.0.len() == 8, view.0.1.len() < 64,
{
    (absorb(view.0, input), view.1 + input.len())
}

pub(super) closed spec fn initial_view() -> ((Seq<u32>, Seq<u8>), nat) {
    ((INITIAL_STATE@, Seq::empty()), 0)
}

closed spec fn bit_length_byte(bit_len: u64, index: int) -> u8
    recommends 0 <= index < 8,
{
    let shift = (56 - index * 8) as u32;
    ((bit_len >> shift) % 256) as u8
}

closed spec fn padded_tail_len(view: ((Seq<u32>, Seq<u8>), nat)) -> nat {
    if view.0.1.len() < 56 { 64 } else { 128 }
}

closed spec fn padded_tail_byte(view: ((Seq<u32>, Seq<u8>), nat), index: int) -> u8
    recommends
        view.0.0.len() == 8,
        view.0.1.len() < 64,
        view.1 <= u64::MAX / 8,
        0 <= index < padded_tail_len(view),
{
    let length_offset = padded_tail_len(view) - 8;
    if index < view.0.1.len() {
        view.0.1[index]
    } else if index == view.0.1.len() {
        0x80
    } else if index < length_offset {
        0
    } else {
        bit_length_byte(
            (view.1 * 8) as u64,
            index - length_offset,
        )
    }
}

closed spec fn padded_tail(view: ((Seq<u32>, Seq<u8>), nat)) -> Seq<u8>
    recommends
        view.0.0.len() == 8,
        view.0.1.len() < 64,
        view.1 <= u64::MAX / 8,
{
    Seq::new(padded_tail_len(view), |index: int| padded_tail_byte(view, index))
}

closed spec fn finish_state(view: ((Seq<u32>, Seq<u8>), nat)) -> Seq<u32>
    recommends
        view.0.0.len() == 8,
        view.0.1.len() < 64,
        view.1 <= u64::MAX / 8,
{
    let tail = padded_tail(view);
    if tail.len() == 64 {
        compress_block(view.0.0, tail)
    } else {
        compress_block(
            compress_block(view.0.0, tail.subrange(0, 64)),
            tail.subrange(64, 128),
        )
    }
}

closed spec fn digest_bytes(state: Seq<u32>) -> Seq<u8>
    recommends state.len() == 8,
{
    Seq::new(32, |index: int| {
        let word = state[index / 4];
        if index % 4 == 0 {
            ((word >> 24) % 256) as u8
        } else if index % 4 == 1 {
            ((word >> 16) % 256) as u8
        } else if index % 4 == 2 {
            ((word >> 8) % 256) as u8
        } else {
            (word % 256) as u8
        }
    })
}

pub(super) closed spec fn finish_view(view: ((Seq<u32>, Seq<u8>), nat)) -> Seq<u8>
    recommends
        view.0.0.len() == 8,
        view.0.1.len() < 64,
        view.1 <= u64::MAX / 8,
{
    digest_bytes(finish_state(view))
}

pub(super) closed spec fn digest_spec(bytes: Seq<u8>) -> Seq<u8> {
    finish_view(update_view(initial_view(), bytes))
}

const INITIAL_STATE: [u32; 8] = [
    0x6a09_e667,
    0xbb67_ae85,
    0x3c6e_f372,
    0xa54f_f53a,
    0x510e_527f,
    0x9b05_688c,
    0x1f83_d9ab,
    0x5be0_cd19,
];

const ROUND_CONSTANTS: [u32; 64] = [
    0x428a_2f98,
    0x7137_4491,
    0xb5c0_fbcf,
    0xe9b5_dba5,
    0x3956_c25b,
    0x59f1_11f1,
    0x923f_82a4,
    0xab1c_5ed5,
    0xd807_aa98,
    0x1283_5b01,
    0x2431_85be,
    0x550c_7dc3,
    0x72be_5d74,
    0x80de_b1fe,
    0x9bdc_06a7,
    0xc19b_f174,
    0xe49b_69c1,
    0xefbe_4786,
    0x0fc1_9dc6,
    0x240c_a1cc,
    0x2de9_2c6f,
    0x4a74_84aa,
    0x5cb0_a9dc,
    0x76f9_88da,
    0x983e_5152,
    0xa831_c66d,
    0xb003_27c8,
    0xbf59_7fc7,
    0xc6e0_0bf3,
    0xd5a7_9147,
    0x06ca_6351,
    0x1429_2967,
    0x27b7_0a85,
    0x2e1b_2138,
    0x4d2c_6dfc,
    0x5338_0d13,
    0x650a_7354,
    0x766a_0abb,
    0x81c2_c92e,
    0x9272_2c85,
    0xa2bf_e8a1,
    0xa81a_664b,
    0xc24b_8b70,
    0xc76c_51a3,
    0xd192_e819,
    0xd699_0624,
    0xf40e_3585,
    0x106a_a070,
    0x19a4_c116,
    0x1e37_6c08,
    0x2748_774c,
    0x34b0_bcb5,
    0x391c_0cb3,
    0x4ed8_aa4a,
    0x5b9c_ca4f,
    0x682e_6ff3,
    0x748f_82ee,
    0x78a5_636f,
    0x84c8_7814,
    0x8cc7_0208,
    0x90be_fffa,
    0xa450_6ceb,
    0xbef9_a3f7,
    0xc671_78f2,
];

pub(super) struct Sha256 {
    state: [u32; 8],
    buffer: [u8; 64],
    buffered: usize,
    byte_len: u64,
}

impl Sha256 {
    pub(super) closed spec fn view(&self) -> ((Seq<u32>, Seq<u8>), nat) {
        (
            (self.state@, self.buffer@.subrange(0, self.buffered as int)),
            self.byte_len as nat,
        )
    }

    pub(super) closed spec fn valid(&self) -> bool {
        &&& self.buffered < 64
        &&& self.byte_len <= u64::MAX / 8
    }

    pub(super) closed spec fn can_update(&self, additional: nat) -> bool {
        &&& additional <= u64::MAX
        &&& self.byte_len as nat + additional <= u64::MAX / 8
    }

    pub(super) const fn new() -> (hasher: Self)
        ensures
            hasher.valid(),
            hasher.can_update((u64::MAX / 8) as nat),
            hasher.view() == initial_view(),
    {
        let hasher = Self {
            state: INITIAL_STATE,
            buffer: [0; 64],
            buffered: 0,
            byte_len: 0,
        };
        assert(hasher.buffer@.subrange(0, 0) == Seq::<u8>::empty());
        hasher
    }

    pub(super) fn update(&mut self, bytes: &[u8])
        requires
            old(self).valid(),
            old(self).can_update(bytes@.len()),
        ensures
            final(self).valid(),
            final(self).view() == update_view(old(self).view(), bytes@),
    {
        let additional = bytes.len();
        let ghost initial_core = self.view().0;
        let ghost expected_core = absorb(initial_core, bytes@);
        let ghost initial_byte_len = self.byte_len;
        assert(bytes@.subrange(0, bytes@.len() as int) == bytes@);
        self.byte_len = self
            .byte_len
            .checked_add(u64::try_from(additional).expect("slice length fits u64"))
            .expect("SHA-256 input length fits u64");
        let mut offset = 0;
        while offset < additional
            invariant
                offset <= additional,
                additional == bytes.len(),
                self.buffered < 64,
                self.byte_len <= u64::MAX / 8,
                self.byte_len as nat == initial_byte_len as nat + additional,
                self.view().0.0.len() == 8,
                absorb(
                    self.view().0,
                    bytes@.subrange(offset as int, additional as int),
                ) == expected_core,
            decreases additional - offset,
        {
            let buffer_index = self.buffered;
            let ghost core_before = self.view().0;
            let ghost remaining_input =
                bytes@.subrange(offset as int, additional as int);
            let ghost filled = core_before.1.push(bytes@[offset as int]);
            let ghost next_core = if filled.len() == 64 {
                (compress_block(core_before.0, filled), Seq::empty())
            } else {
                (core_before.0, filled)
            };
            assert(remaining_input.len() != 0);
            assert(remaining_input[0] == bytes@[offset as int]);
            assert(remaining_input.subrange(1, remaining_input.len() as int)
                == bytes@.subrange((offset + 1) as int, additional as int));
            self.buffer[buffer_index] = bytes[offset];
            self.buffered += 1;
            assert(self.view().0.1 == filled);
            offset += 1;
            if self.buffered == 64 {
                assert(filled.len() == 64);
                assert(self.buffer@ == filled);
                let block = self.buffer;
                self.compress(&block);
                assert(self.state@ == compress_block(core_before.0, filled));
                self.buffered = 0;
                assert(self.buffer@.subrange(0, 0) == Seq::<u8>::empty());
            } else {
                assert(filled.len() != 64);
            }
            assert(self.view().0 == next_core);
            assert(absorb(core_before, remaining_input)
                == absorb(
                    next_core,
                    bytes@.subrange(offset as int, additional as int),
                ));
        }
    }

    pub(super) fn finish(self) -> (digest: [u8; 32])
        requires self.valid(),
        ensures digest@ == finish_view(self.view()),
    {
        let ghost original_view = self.view();
        assert(original_view.0.0.len() == 8);
        assert(original_view.0.1.len() < 64);
        assert(original_view.1 <= u64::MAX / 8);
        let mut this = self;
        let bit_len = this
            .byte_len
            .checked_mul(8)
            .expect("SHA-256 bit length fits u64");
        let tail_len = if this.buffered < 56 { 64 } else { 128 };
        let mut tail = [0_u8; 128];
        let mut index = 0;
        while index < tail_len
            invariant
                index <= tail_len,
                tail_len <= 128,
                tail_len == padded_tail_len(original_view),
                this.view() == original_view,
                this.buffered < 64,
                original_view.0.0.len() == 8,
                original_view.0.1.len() < 64,
                original_view.1 <= u64::MAX / 8,
                bit_len as nat == original_view.1 * 8,
                bit_len == (original_view.1 * 8) as u64,
                tail@.subrange(0, index as int)
                    == padded_tail(original_view).subrange(0, index as int),
            decreases tail_len - index,
        {
            let byte = if index < this.buffered {
                assert((index as int) < original_view.0.1.len());
                this.buffer[index]
            } else if index == this.buffered {
                0x80
            } else if index < tail_len - 8 {
                0
            } else {
                let length_index = index - (tail_len - 8);
                assert(length_index < 8);
                let length_byte = (bit_len >> (56 - length_index * 8)) % 256;
                let encoded = u8::try_from(length_byte).expect("length byte fits u8");
                assert(encoded == bit_length_byte(bit_len, length_index as int));
                encoded
            };
            assert(byte == padded_tail_byte(original_view, index as int));
            assert(padded_tail(original_view)[index as int]
                == padded_tail_byte(original_view, index as int));
            tail[index] = byte;
            index += 1;
        }
        assert(tail@.subrange(0, tail_len as int) == padded_tail(original_view));

        let mut block = [0_u8; 64];
        index = 0;
        while index < 64
            invariant
                index <= 64,
                block@.subrange(0, index as int)
                    == tail@.subrange(0, index as int),
            decreases 64 - index,
        {
            block[index] = tail[index];
            index += 1;
        }
        assert(block@ == padded_tail(original_view).subrange(0, 64));
        this.compress(&block);
        if tail_len == 128 {
            assert(tail@.subrange(64, 128)
                == padded_tail(original_view).subrange(64, 128));
            index = 0;
            while index < 64
                invariant
                    index <= 64,
                    block@.subrange(0, index as int)
                        == tail@.subrange(64, (64 + index) as int),
                decreases 64 - index,
            {
                block[index] = tail[64 + index];
                index += 1;
            }
            assert(block@ == padded_tail(original_view).subrange(64, 128));
            this.compress(&block);
        }
        assert(this.state@ == finish_state(original_view));

        let mut digest = [0; 32];
        index = 0;
        while index < 32
            invariant
                index <= 32,
                this.state@ == finish_state(original_view),
                original_view.0.0.len() == 8,
                original_view.0.1.len() < 64,
                original_view.1 <= u64::MAX / 8,
                digest@.subrange(0, index as int)
                    == digest_bytes(this.state@).subrange(0, index as int),
            decreases 32 - index,
        {
            let word = this.state[index / 4];
            let byte = if index % 4 == 0 {
                u8::try_from((word >> 24) % 256).expect("digest byte fits u8")
            } else if index % 4 == 1 {
                u8::try_from((word >> 16) % 256).expect("digest byte fits u8")
            } else if index % 4 == 2 {
                u8::try_from((word >> 8) % 256).expect("digest byte fits u8")
            } else {
                u8::try_from(word % 256).expect("digest byte fits u8")
            };
            digest[index] = byte;
            index += 1;
        }
        assert(digest@.subrange(0, 32) == digest@);
        assert(digest_bytes(finish_state(original_view)).len() == 32);
        assert(digest_bytes(finish_state(original_view)).subrange(0, 32)
            == digest_bytes(finish_state(original_view)));
        assert(digest@ == digest_bytes(finish_state(original_view)));
        assert(finish_view(original_view) == digest_bytes(finish_state(original_view)));
        digest
    }

    fn compress(&mut self, block: &[u8; 64])
        ensures
            final(self).state@ == compress_block(old(self).state@, block@),
            final(self).buffer@ == old(self).buffer@,
            final(self).buffered == old(self).buffered,
            final(self).byte_len == old(self).byte_len,
    {
        let mut schedule = [0_u32; 64];
        let mut index = 0;
        while index < 16
            invariant
                index <= 16,
                schedule@.subrange(0, index as int)
                    == schedule_prefix(block@, index as nat),
            decreases 16 - index,
        {
            let offset = index * 4;
            schedule[index] = (u32::from(block[offset]) << 24)
                | (u32::from(block[offset + 1]) << 16)
                | (u32::from(block[offset + 2]) << 8)
                | u32::from(block[offset + 3]);
            index += 1;
        }
        while index < 64
            invariant
                16 <= index <= 64,
                schedule@.subrange(0, index as int)
                    == schedule_prefix(block@, index as nat),
            decreases 64 - index,
        {
            let sigma0 = rotate_right(schedule[index - 15], 7)
                ^ rotate_right(schedule[index - 15], 18)
                ^ (schedule[index - 15] >> 3);
            let sigma1 = rotate_right(schedule[index - 2], 17)
                ^ rotate_right(schedule[index - 2], 19)
                ^ (schedule[index - 2] >> 10);
            schedule[index] = schedule[index - 16]
                .wrapping_add(sigma0)
                .wrapping_add(schedule[index - 7])
                .wrapping_add(sigma1);
            index += 1;
        }

        assert(schedule@ == schedule_prefix(block@, 64));
        let ghost original_state = self.state@;
        let mut state_a = self.state[0];
        let mut state_b = self.state[1];
        let mut state_c = self.state[2];
        let mut state_d = self.state[3];
        let mut state_e = self.state[4];
        let mut state_f = self.state[5];
        let mut state_g = self.state[6];
        let mut state_h = self.state[7];
        index = 0;
        while index < 64
            invariant
                index <= 64,
                schedule@ == schedule_prefix(block@, 64),
                state_a == rounds(
                    initial_working_state(original_state), schedule@, index as nat,
                ).0,
                state_b == rounds(
                    initial_working_state(original_state), schedule@, index as nat,
                ).1,
                state_c == rounds(
                    initial_working_state(original_state), schedule@, index as nat,
                ).2,
                state_d == rounds(
                    initial_working_state(original_state), schedule@, index as nat,
                ).3,
                state_e == rounds(
                    initial_working_state(original_state), schedule@, index as nat,
                ).4,
                state_f == rounds(
                    initial_working_state(original_state), schedule@, index as nat,
                ).5,
                state_g == rounds(
                    initial_working_state(original_state), schedule@, index as nat,
                ).6,
                state_h == rounds(
                    initial_working_state(original_state), schedule@, index as nat,
                ).7,
            decreases 64 - index,
        {
            let sum1 =
                rotate_right(state_e, 6) ^ rotate_right(state_e, 11) ^ rotate_right(state_e, 25);
            let choice = (state_e & state_f) ^ (!state_e & state_g);
            let temporary1 = state_h
                .wrapping_add(sum1)
                .wrapping_add(choice)
                .wrapping_add(ROUND_CONSTANTS[index])
                .wrapping_add(schedule[index]);
            let sum0 =
                rotate_right(state_a, 2) ^ rotate_right(state_a, 13) ^ rotate_right(state_a, 22);
            let majority = (state_a & state_b) ^ (state_a & state_c) ^ (state_b & state_c);
            let temporary2 = sum0.wrapping_add(majority);
            state_h = state_g;
            state_g = state_f;
            state_f = state_e;
            state_e = state_d.wrapping_add(temporary1);
            state_d = state_c;
            state_c = state_b;
            state_b = state_a;
            state_a = temporary1.wrapping_add(temporary2);
            index += 1;
        }

        let values = [
            state_a, state_b, state_c, state_d, state_e, state_f, state_g, state_h,
        ];
        index = 0;
        while index < 8
            invariant
                index <= 8,
                original_state.len() == 8,
                values@.len() == 8,
                forall|prior: int| 0 <= prior < index ==>
                    self.state@[prior]
                        == original_state[prior].wrapping_add(values@[prior]),
                forall|remaining: int| index <= remaining < 8 ==>
                    self.state@[remaining] == original_state[remaining],
            decreases 8 - index,
        {
            self.state[index] = self.state[index].wrapping_add(values[index]);
            index += 1;
        }
    }
}

pub(super) fn digest(bytes: &[u8]) -> (digest: [u8; 32])
    requires bytes@.len() <= u64::MAX / 8,
    ensures digest@ == digest_spec(bytes@),
{
    let mut hasher = Sha256::new();
    assert(hasher.can_update(bytes@.len()));
    hasher.update(bytes);
    hasher.finish()
}

}
