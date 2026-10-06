macro_rules! equal_state_words_v1 {
    ($state:ident, $valid:ident, $load:ident; $expected:expr; $($index:literal),+ $(,)?) => {
        $(
            let word: u32 = $load!($state, $index);
            $valid = $valid & (word == $expected);
        )+
    };
}

macro_rules! owner_state_words_v1 {
    ($state:ident, $valid:ident, $load:ident; $($index:literal),+ $(,)?) => {
        $(
            let owner: u32 = $load!($state, $index);
            $valid = $valid & (owner >= 1) & (owner <= 64);
        )+
    };
}

// Both extent-checked entry macros reuse the unchanged literal state predicate.
// This helper receives the original slice, never a projected prefix/subslice.
macro_rules! mlp_state_words_valid_v2 {
    ($state:expr, $load:ident) => {{
        let state = $state;
        let mut valid = true;
        equal_state_words_v1!(state, valid, $load; 1; 0, 4, 7, 9, 12);
        equal_state_words_v1!(state, valid, $load; 0; 1, 2);
        equal_state_words_v1!(state, valid, $load; 31; 3);
        equal_state_words_v1!(state, valid, $load; 96; 5, 6, 10, 11);
        equal_state_words_v1!(state, valid, $load; 64; 8, 13);
        equal_state_words_v1!(state, valid, $load; u32::MAX;
            14, 15, 16, 17, 18, 19, 20, 21,
            23, 24, 25, 26, 27, 28, 29, 30);
        equal_state_words_v1!(state, valid, $load; 3; 22, 31);
        owner_state_words_v1!(state, valid, $load;
        32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47,
        48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63,
        64, 65, 66, 67, 68, 69, 70, 71, 72, 73, 74, 75, 76, 77, 78, 79,
        80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90, 91, 92, 93, 94, 95,
        96, 97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111,
        112, 113, 114, 115, 116, 117, 118, 119, 120, 121, 122, 123, 124, 125, 126, 127,
        128, 129, 130, 131, 132, 133, 134, 135, 136, 137, 138, 139, 140, 141, 142, 143,
        144, 145, 146, 147, 148, 149, 150, 151, 152, 153, 154, 155, 156, 157, 158, 159,
        160, 161, 162, 163, 164, 165, 166, 167, 168, 169, 170, 171, 172, 173, 174, 175,
        176, 177, 178, 179, 180, 181, 182, 183, 184, 185, 186, 187, 188, 189, 190, 191,
        192, 193, 194, 195, 196, 197, 198, 199, 200, 201, 202, 203, 204, 205, 206, 207,
        208, 209, 210, 211, 212, 213, 214, 215, 216, 217, 218, 219, 220, 221, 222, 223,
        224, 225, 226, 227, 228, 229, 230, 231, 232, 233, 234, 235, 236, 237, 238, 239,
        240, 241, 242, 243, 244, 245, 246, 247, 248, 249, 250, 251, 252, 253, 254, 255,
        256, 257, 258, 259, 260, 261, 262, 263, 264, 265, 266, 267, 268, 269, 270, 271,
        272, 273, 274, 275, 276, 277, 278, 279, 280, 281, 282, 283, 284, 285, 286, 287,
        288, 289);
        equal_state_words_v1!(state, valid, $load; 64;
        290, 291, 292, 293, 294, 295, 296, 297, 298, 299, 300, 301, 302, 303, 304, 305,
        306, 307, 308, 309, 310, 311, 312, 313, 314, 315, 316, 317, 318, 319, 320, 321,
        322, 323, 324, 325, 326, 327, 328, 329, 330, 331, 332, 333, 334, 335, 336, 337,
        338, 339, 340, 341, 342, 343, 344, 345, 346, 347, 348, 349, 350, 351, 352, 353,
        354, 355, 356, 357, 358, 359, 360, 361, 362, 363, 364, 365, 366, 367, 368, 369,
        370, 371, 372, 373, 374, 375, 376, 377, 378, 379, 380, 381, 382, 383, 384, 385,
        386, 387, 388, 389, 390, 391, 392, 393, 394, 395, 396, 397, 398, 399, 400, 401,
        402, 403, 404, 405, 406, 407, 408, 409, 410, 411, 412, 413, 414, 415, 416, 417,
        418, 419, 420, 421, 422, 423, 424, 425, 426, 427, 428, 429, 430, 431, 432, 433,
        434, 435, 436, 437, 438, 439, 440, 441, 442, 443, 444, 445, 446, 447, 448, 449,
        450, 451, 452, 453, 454, 455, 456, 457, 458, 459, 460, 461, 462, 463, 464, 465,
        466, 467, 468, 469, 470, 471, 472, 473, 474, 475, 476, 477, 478, 479, 480, 481,
        482, 483, 484, 485, 486, 487, 488, 489, 490, 491, 492, 493, 494, 495, 496, 497,
        498, 499, 500, 501, 502, 503, 504, 505, 506, 507, 508, 509, 510, 511, 512, 513,
        514, 515, 516, 517, 518, 519, 520, 521, 522, 523, 524, 525, 526, 527, 528, 529,
        530, 531, 532, 533, 534, 535, 536, 537, 538, 539, 540, 541, 542, 543, 544, 545,
        546, 547);
        valid
    }};
}

// Every state word is loaded exactly once through a literal atomic index.
macro_rules! mlp_state_valid_v1 {
    ($state:expr, $load:ident) => {{
        let state = $state;
        if state.len() != 548 {
            false
        } else {
            mlp_state_words_valid_v2!(state, $load)
        }
    }};
}

// A single source argument covers state [0, 548) and guard [548, 552).
// The validator never reads the suffix and never fabricates a second view.
macro_rules! mlp_combined_state_valid_v2 {
    ($state_guard:expr, $load:ident) => {{
        let state_guard = $state_guard;
        if state_guard.len() != 552 {
            false
        } else {
            mlp_state_words_valid_v2!(state_guard, $load)
        }
    }};
}

// Preserve the old four-word wire order at its literal combined offsets:
// low, high, reserved (Relaxed), then verdict (Release) at index 550.
macro_rules! publish_combined_guard_v2 {
    ($state_guard:expr, $lo:expr, $hi:expr, $valid:expr, $store:ident) => {{
        let state_guard = $state_guard;
        let lo: u32 = $lo;
        let hi: u32 = $hi;
        let valid: bool = $valid;
        if state_guard.len() != 552 || (lo | hi) == 0 {
            fe2o3_device::trap();
        }
        $store!(state_guard, 548, lo, core::sync::atomic::Ordering::Relaxed);
        $store!(state_guard, 549, hi, core::sync::atomic::Ordering::Relaxed);
        $store!(state_guard, 551, 0, core::sync::atomic::Ordering::Relaxed);
        let verdict = if valid { 1_u32 } else { 2_u32 };
        $store!(state_guard, 550, verdict, core::sync::atomic::Ordering::Release);
    }};
}

// Verdict is published last. The runtime must own generation uniqueness and
// prevent reset or concurrent writers until the previous consumer completes.
macro_rules! publish_guard_v1 {
    ($guard:expr, $lo:expr, $hi:expr, $valid:expr, $store:ident) => {{
        let guard = $guard;
        let lo: u32 = $lo;
        let hi: u32 = $hi;
        let valid: bool = $valid;
        if guard.len() != 4 || (lo | hi) == 0 {
            fe2o3_device::trap();
        }
        $store!(guard, 0, lo, core::sync::atomic::Ordering::Relaxed);
        $store!(guard, 1, hi, core::sync::atomic::Ordering::Relaxed);
        $store!(guard, 3, 0, core::sync::atomic::Ordering::Relaxed);
        let verdict = if valid { 1_u32 } else { 2_u32 };
        $store!(guard, 2, verdict, core::sync::atomic::Ordering::Release);
    }};
}

macro_rules! guard_current_v1 {
    ($guard:expr, $lo:expr, $hi:expr, $load:ident) => {{
        let guard = $guard;
        let lo: u32 = $lo;
        let hi: u32 = $hi;
        if guard.len() != 4 {
            false
        } else {
            // Acquire verdict precedes the fields covered by its release.
            let verdict: u32 = $load!(guard, 2);
            let actual_lo: u32 = $load!(guard, 0);
            let actual_hi: u32 = $load!(guard, 1);
            let reserved: u32 = $load!(guard, 3);
            ((lo | hi) != 0)
                & (verdict == 1)
                & (actual_lo == lo)
                & (actual_hi == hi)
                & (reserved == 0)
        }
    }};
}

macro_rules! with_current_guards_v1 {
    ($guard0:expr, $guard1:expr, $lo:expr, $hi:expr, $load:ident;
     $payload:block) => {{
        let lo: u32 = $lo;
        let hi: u32 = $hi;
        let valid0 = guard_current_v1!($guard0, lo, hi, $load);
        let valid1 = guard_current_v1!($guard1, lo, hi, $load);
        if valid0 & valid1 {
            $payload
        }
    }};
}
