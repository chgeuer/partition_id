use clap::Parser;
use std::num::Wrapping;

#[inline]
fn rot(x: Wrapping<u32>, k: usize) -> Wrapping<u32> {
    x << k | x >> (32 - k)
}

#[inline]
fn mix(a: &mut Wrapping<u32>, b: &mut Wrapping<u32>, c: &mut Wrapping<u32>) {
    *a -= *c;
    *a ^= rot(*c, 4);
    *c += *b;
    *b -= *a;
    *b ^= rot(*a, 6);
    *a += *c;
    *c -= *b;
    *c ^= rot(*b, 8);
    *b += *a;
    *a -= *c;
    *a ^= rot(*c, 16);
    *c += *b;
    *b -= *a;
    *b ^= rot(*a, 19);
    *a += *c;
    *c -= *b;
    *c ^= rot(*b, 4);
    *b += *a;
}

#[inline]
fn final_mix(a: &mut Wrapping<u32>, b: &mut Wrapping<u32>, c: &mut Wrapping<u32>) {
    *c ^= *b;
    *c -= rot(*b, 14);
    *a ^= *c;
    *a -= rot(*c, 11);
    *b ^= *a;
    *b -= rot(*a, 25);
    *c ^= *b;
    *c -= rot(*b, 16);
    *a ^= *c;
    *a -= rot(*c, 4);
    *b ^= *a;
    *b -= rot(*a, 14);
    *c ^= *b;
    *c -= rot(*b, 24);
}

#[inline]
fn shift_add(s: &[u8]) -> Wrapping<u32> {
    Wrapping(match s.len() {
        4 => (s[0] as u32) + ((s[1] as u32) << 8) + ((s[2] as u32) << 16) + ((s[3] as u32) << 24),
        3 => (s[0] as u32) + ((s[1] as u32) << 8) + ((s[2] as u32) << 16),
        2 => (s[0] as u32) + ((s[1] as u32) << 8),
        1 => s[0] as u32,
        _ => 0 as u32,
    })
}

fn hash(bytes: &[u8]) -> (u32, u32, u64) {
    let pc = Wrapping(0u32);
    let pb = Wrapping(0u32);
    let initial = Wrapping(0xdeadbeefu32) + Wrapping(bytes.len() as u32) + pc;
    let mut a = initial;
    let mut b = initial;
    let mut c = initial;
    c += pb;

    let full_mix_rounds = (bytes.len() - 1) / 12;
    let mut fully_mixed = 0_usize;

    for chunk in bytes.chunks(12) {
        let size = chunk.len();
        match size {
            12 => {
                c += shift_add(&chunk[8..]);
                b += shift_add(&chunk[4..8]);
                a += shift_add(&chunk[..4]);

                if fully_mixed < full_mix_rounds {
                    mix(&mut a, &mut b, &mut c);
                    fully_mixed += 1;
                }
            }
            11 | 10 | 9 | 8 => {
                c += shift_add(&chunk[8..]);
                b += shift_add(&chunk[4..8]);
                a += shift_add(&chunk[..4]);
            }
            7 | 6 | 5 | 4 => {
                b += shift_add(&chunk[4..]);
                a += shift_add(&chunk[..4]);
            }
            3 | 2 | 1 => {
                a += shift_add(chunk);
            }
            0 => {
                return (c.0, b.0, (c.0 as u64) + ((b.0 as u64) << 32));
            }
            _ => {}
        }
    }

    final_mix(&mut a, &mut b, &mut c);

    (c.0, b.0, (c.0 as u64) + ((b.0 as u64) << 32))
}

fn get_ranges(range_count: i16) -> Vec<i32> {
    let mut ranges = Vec::with_capacity(range_count as usize);

    let count = i16::max_value();
    let partitions_per_range_base = count / range_count;
    let remaining_partitions = count - (range_count * partitions_per_range_base);

    let mut end = -1;
    for i in 0..range_count - 1 {
        let partitions_per_range = if i < remaining_partitions {
            partitions_per_range_base + 1
        } else {
            partitions_per_range_base
        };

        end = i32::min(end + partitions_per_range as i32, count as i32 - 1);
        ranges.push(end);
    }

    ranges.push(count as i32 - 1);

    ranges
}

fn to_logical(partition_key: String) -> i32 {
    if partition_key.is_empty() {
        return 0;
    }
    let (hash1, hash2, _x) = hash(partition_key.to_uppercase().as_bytes());

    ((hash1 ^ hash2) % 32767u32) as i32
}

fn to_partition_id(ranges: &[i32], partition: i32) -> u16 {
    let mut lower = 0;
    let mut upper = ranges.len() - 1;
    while lower < upper {
        let middle = (lower + upper) >> 1;

        if partition > ranges[middle] {
            lower = middle + 1;
        } else {
            upper = middle;
        }
    }

    lower as u16
}

fn get_partition_id(partition_count: i16, partition_key: &str) -> u16 {
    to_partition_id(
        &get_ranges(partition_count),
        to_logical(partition_key.to_uppercase()),
    )
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Number of partitions
    #[arg(short = 'c', long)]
    partition_count: i16,

    /// Partition Key
    #[arg(short = 'k', long)]
    partition_key: String,
}

fn main() {
    let args = Args::parse();

    println!(
        "{}",
        get_partition_id(args.partition_count, args.partition_key.as_str())
    );
}
