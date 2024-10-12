use std::ops::Index;

pub struct LogArray {
    data: Vec<u64>,
    width: u8,
    length: usize,
}

/// Construction
impl LogArray {
    const fn required_data_len(width: u8, length: usize) -> usize {
        // avoid integer overflows by going to u128
        let bit_length = length as u128 * width as u128;
        let u64_length = (bit_length + 63) / 64;

        u64_length as usize
    }

    pub fn new(width: u8, length: usize) -> LogArray {
        let data_len = Self::required_data_len(width, length);
        let data = vec![0; data_len];

        LogArray {
            data,
            width,
            length,
        }
    }
}

#[derive(Clone, Copy)]
struct LogArrayPos {
    u64_index: usize,
    offset: u8,
}

#[derive(Clone, Copy)]
struct LogArrayMask {
    mask: u64,
    shift: u8,
}

/// access
const fn pos(width: u8, index: usize) -> LogArrayPos {
    let bit_index = index as u128 * width as u128;
    let u64_index = (bit_index / 64) as usize;
    let offset = (bit_index % 64) as u8;

    LogArrayPos { u64_index, offset }
}

const fn shift_mask_1(width: u8, offset: u8) -> LogArrayMask {
    debug_assert!(offset + width <= 64);

    let mut mask: u64 = if width == 64 {
        // light it all up
        !0
    } else {
        (1 << width) - 1
    };

    let shift = 64 - offset - width;

    mask = mask.rotate_left(shift as u32);

    LogArrayMask { mask, shift }
}

const fn shift_mask_2(width: u8, offset: u8) -> (LogArrayMask, LogArrayMask) {
    debug_assert!(offset + width > 64);
    let width1 = 64 - offset;
    let width2 = width - width1;

    let mask1: u64 = (1 << width1) - 1;
    let mask2: u64 = !((1 << (64 - width2)) - 1);

    let shift1 = width2;
    let shift2 = 64 - width2;

    (
        LogArrayMask {
            mask: mask1,
            shift: shift1,
        },
        LogArrayMask {
            mask: mask2,
            shift: shift2,
        },
    )
}

impl LogArray {
    pub fn load(&self, index: usize) -> u64 {
        assert!(index < self.length);
        let LogArrayPos { u64_index, offset } = pos(self.width, index);
        if offset + self.width <= 64 {
            // everything fits within one entry
            let LogArrayMask { mask, shift } = shift_mask_1(self.width, offset);
            let value_shifted = self.data[u64_index];
            (value_shifted & mask) >> shift
        } else {
            // crosses over into next entry
            let (
                LogArrayMask {
                    mask: mask1,
                    shift: shift1,
                },
                LogArrayMask {
                    // don't need mask if we shift it all away
                    mask: _,
                    shift: shift2,
                },
            ) = shift_mask_2(self.width, offset);

            let value_shifted1 = self.data[u64_index];
            let value_1 = (value_shifted1 & mask1) << shift1;
            let value_shifted2 = self.data[u64_index + 1];
            let value_2 = value_shifted2 >> shift2;

            value_1 | value_2
        }
    }
    pub fn store(&mut self, index: usize, value: u64) {
        assert!(index < self.length);
        assert!((64 - value.leading_zeros()) as u8 <= self.width);
        let LogArrayPos { u64_index, offset } = pos(self.width, index);
        if offset + self.width <= 64 {
            // everything fits within one entry
            let LogArrayMask { mask, shift } = shift_mask_1(self.width, offset);
            let value_shifted = value << shift;
            self.data[u64_index] &= !mask;
            self.data[u64_index] |= value_shifted;
        } else {
            // crosses over into next entry
            let (
                LogArrayMask {
                    mask: mask1,
                    shift: shift1,
                },
                LogArrayMask {
                    mask: mask2,
                    shift: shift2,
                },
            ) = shift_mask_2(self.width, offset);

            let value_shifted1 = value >> shift1;
            self.data[u64_index] &= !mask1;
            self.data[u64_index] |= value_shifted1;

            let value_shifted2 = value << shift2;
            self.data[u64_index + 1] &= !mask2;
            self.data[u64_index + 1] |= value_shifted2;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_load_cycle() {
        let mut logarray = LogArray::new(10, 1000);
        for i in 0..1000 {
            logarray.store(i, i as u64);
        }
        for i in 0..1000 {
            assert_eq!(i as u64, logarray.load(i));
        }
    }

    #[test]
    fn store_load_check_neighbors() {
        let mut logarray = LogArray::new(10, 1000);
        for i in 0..1000 {
            logarray.store(i, i as u64);
        }

        // make sure we can overwrite each element without affecting its neighbors
        for i in 0..1000 {
            // overwrite with an out of band
            logarray.store(i, 1001);
            assert_eq!(1001, logarray.load(i));
            if i != 0 {
                assert_eq!((i - 1) as u64, logarray.load(i - 1));
            }
            if i != 999 {
                assert_eq!((i + 1) as u64, logarray.load(i + 1));
            }
            // restore original value
            logarray.store(i, i as u64);
        }
    }
}
