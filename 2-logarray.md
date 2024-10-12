# The logarray
In the last article I described the basic idea behind succinct data
structures, and explained the first one that I want to go over in this
series, the logarray.

In this article I am going to actually implement this. I'll start with
a naive version to get the basic interface, tests and benchmarks
down. Then we'll go through some iterations of improvements, both in
terms of performance, and in terms of programming ergonomics.

All this code will be rust. I'll provide the relevant code snippets
inline here, but they can also be found on github.

## A naive implementation
Let's get the most basic thing down that we want this to be. Remember
that the basic principle of the logarray is that we reduce each entry
to the minimum amount of bits that could still hold the largest value,
then squish those together into a large bit array. So how do you
implement that?

It would be great if we could directly ask a CPU to get us a
particular bit range. Unfortunately, modern CPUs do not let us do
that. instead, modern CPUs let us efficiently look at memory in
various increments, those being 1, 2, 4 or 8 byte increments. Or if we
think about this in terms of bits, that is at 8, 16, 32, and 64 bit
increments. Only values that start exactly at a multiple of one of
those increments can be retrieved efficiently by that.

Furthermore, this increment also influences how much is looked up 'at
once'. For most basic instructions, the CPU can load 1, 2, 4 or 8
bytes at once. So while a 1 byte/8 bit increment gives us more
coverage, since we can only do one 8 bit load with that increment,
we'll need to do several to get the entire number out if it is any
wider than 8 bits.

No matter what size we pick though, we'll have to deal with the issue
that for many choices of widths, we'll have entries that will only
span part of a access, or might require several accesses worth of
data. We'll need an encoder and decoder to turn logarray indexes into
the actual required memory accesses.

### Structure
Let's get a basic data structure down.
```rust
pub struct LogArray {
    data: Vec<u64>,
    width: u8,
    length: usize
}

```

I'm going for `u64` as our machine width. I assume this makes more
sense than a `Vec<u8>`, which as I explained above, will need more
memory accesses to cover the wider widths. My expectation is that
`Vec<u64>` would be faster. However, we should measure this later!

### Construction
```rust
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
```

Since we don't have an encoder and decoder written yet, there isn't
much we can do in construction except just filling a memory range with
zeroes.

Note the carefulness around calculating the length of the data vec. If
we aren't careful, calculations could overflow for large lengths. For
the naive implementation I'll just be using u128 wherever that might
be an issue.

### Access
So how do we work with this data structure? The first thing we need is
some way to tell, for a particular index, which entry in the
`Vec<u64>` actually holds it. Or at least, the start of it, as some
will be crossing over into the adjacent entry. We also need to know at
at which bit index inside that `u64` the requested entry starts.

With that information we can then call an encoder/decoder. This should
figure out how to actually load a `width` worth of bits from that
location. This will involve one or two memory loads, some masking, and
some bit shifting.

#### Figuring out the position
```rust
const fn pos(width: u8, index: usize) -> LogArrayPos {
    let bit_index = index as u128 * width as u128;
    let u64_index = (bit_index / 64) as usize;
    let offset = (bit_index % 64) as u8;

    LogArrayPos { u64_index, offset }
}

```
Again, by casting to u128, we're able to do bit position calculations
without risking an integer overflow. The position and offset are
straightforward with some integer division and modulo.

#### Case 1: value fits within one entry
When the value fits within one entry, there's only two further things
we need to know in order to do loads or stores. First, we need to
figure out how much of the entry is our value, instead of an adjacent
value. We'll return this information as a bit mask. The second thing
we need to know is how much we'd have to shift the entry so that our
desired value appears in the least significant bits.

```rust
#[derive(Clone, Copy)]
struct LogArrayMask {
    mask: u64,
    shift: u8,
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
```

Note that we need to special-case bit width 64, as (1<<64) would
overflow.

#### Case 2: value crosses over into next entry
For values that lie on the edge of an entry, we need to load the bits
from two adjacent entries and then recombine. This means we have to
calculate two masks and two shifts.

```rust
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
```

here we don't need to do any overflow checking, cause we already know
the width is going to be less than 64 (since 64-bit width logarrays
are basically just ordinary aligned ranges of u64 values).

#### Loading and storing
We can now use the calculated index, offset, shifts and masks to
actually load and store things into the logarray.

```rust
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
```

### Some tests
To make sure this actually runs, I made two tests.

`store_load_cycles()` will simply store 1000 values, then makes sure
it can load them back out.

`store_load_check_neighbors()` also stores 1000 values, but then loops
through the entire logarray, setting each value, and making sure the
neighbors are unaffected. This makes sure that if there's some mistake
in all that shifting and masking logic, we're not accidentally
clobbering values.

```rust
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
```

### Benchmarks
And now the moment we've all been waiting for. How does this actually
perform compared to more straightforward vector operations?

In order to find out, let's write some benchmarks. The rust ecosystem
has a nice library for this, called criterion. It can run some
function many times in order to figure out how long on average that
operation takes. We'll be using it to benchmark both stores and loads.
