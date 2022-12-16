use clap::Parser;

fn main() {
    let args = Args::parse();

    println!(
        "{}",
        get_partition_id(args.partition_count, args.partition_key.as_str())
    );
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

use std::num::Wrapping;

#[inline]
fn rot(x: Wrapping<u32>, k: usize) -> Wrapping<u32> {
    x << k | x >> (32 - k)
}

#[rustfmt::skip]
#[inline]
fn mix(a: &mut Wrapping<u32>, b: &mut Wrapping<u32>, c: &mut Wrapping<u32>) {
    *a -= *c; *a ^= rot(*c,  4); *c += *b;
    *b -= *a; *b ^= rot(*a,  6); *a += *c;
    *c -= *b; *c ^= rot(*b,  8); *b += *a;
    *a -= *c; *a ^= rot(*c, 16); *c += *b;
    *b -= *a; *b ^= rot(*a, 19); *a += *c;
    *c -= *b; *c ^= rot(*b,  4); *b += *a;
}

#[rustfmt::skip]
#[inline]
fn final_mix(a: &mut Wrapping<u32>, b: &mut Wrapping<u32>, c: &mut Wrapping<u32>) {
    *c ^= *b; *c -= rot(*b, 14);
    *a ^= *c; *a -= rot(*c, 11);
    *b ^= *a; *b -= rot(*a, 25);
    *c ^= *b; *c -= rot(*b, 16);
    *a ^= *c; *a -= rot(*c,  4);
    *b ^= *a; *b -= rot(*a, 14);
    *c ^= *b; *c -= rot(*b, 24);
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
            // 0 => {
            //     return (c.0, b.0, (c.0 as u64) + ((b.0 as u64) << 32));
            // }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[rustfmt::skip]
    #[test]
    fn test_partitions() -> Result<(), String> {
        assert_eq!(get_partition_id(01, "00000000-0000-0101-9A83-DEADDEADBEEF"), 0);

        assert_eq!(get_partition_id(02, "00000000-0000-0202-94F1-DEADDEADBEEF"), 0);
        assert_eq!(get_partition_id(02, "00000000-0101-0202-B117-DEADDEADBEEF"), 1);

        assert_eq!(get_partition_id(03, "00000000-0000-0303-8BF0-DEADDEADBEEF"), 0);
        assert_eq!(get_partition_id(03, "00000000-0101-0303-BF1B-DEADDEADBEEF"), 1);
        assert_eq!(get_partition_id(03, "00000000-0202-0303-8E30-DEADDEADBEEF"), 2);

        assert_eq!(get_partition_id(04, "00000000-0000-0404-85C7-DEADDEADBEEF"), 0);
        assert_eq!(get_partition_id(04, "00000000-0101-0404-AEB1-DEADDEADBEEF"), 1);
        assert_eq!(get_partition_id(04, "00000000-0202-0404-B6DA-DEADDEADBEEF"), 2);
        assert_eq!(get_partition_id(04, "00000000-0303-0404-8557-DEADDEADBEEF"), 3);

        assert_eq!(get_partition_id(05, "00000000-0000-0505-A086-DEADDEADBEEF"), 0);
        assert_eq!(get_partition_id(05, "00000000-0101-0505-8DAE-DEADDEADBEEF"), 1);
        assert_eq!(get_partition_id(05, "00000000-0202-0505-950D-DEADDEADBEEF"), 2);
        assert_eq!(get_partition_id(05, "00000000-0303-0505-98CA-DEADDEADBEEF"), 3);
        assert_eq!(get_partition_id(05, "00000000-0404-0505-B4C9-DEADDEADBEEF"), 4);

        assert_eq!(get_partition_id(06, "00000000-0000-0606-A73E-DEADDEADBEEF"), 0);
        assert_eq!(get_partition_id(06, "00000000-0101-0606-8BBD-DEADDEADBEEF"), 1);
        assert_eq!(get_partition_id(06, "00000000-0202-0606-A12E-DEADDEADBEEF"), 2);
        assert_eq!(get_partition_id(06, "00000000-0303-0606-B935-DEADDEADBEEF"), 3);
        assert_eq!(get_partition_id(06, "00000000-0404-0606-8D62-DEADDEADBEEF"), 4);
        assert_eq!(get_partition_id(06, "00000000-0505-0606-AE21-DEADDEADBEEF"), 5);

        assert_eq!(get_partition_id(07, "00000000-0000-0707-AF8B-DEADDEADBEEF"), 0);
        assert_eq!(get_partition_id(07, "00000000-0101-0707-A48B-DEADDEADBEEF"), 1);
        assert_eq!(get_partition_id(07, "00000000-0202-0707-B9EC-DEADDEADBEEF"), 2);
        assert_eq!(get_partition_id(07, "00000000-0303-0707-961B-DEADDEADBEEF"), 3);
        assert_eq!(get_partition_id(07, "00000000-0404-0707-8B09-DEADDEADBEEF"), 4);
        assert_eq!(get_partition_id(07, "00000000-0505-0707-83B8-DEADDEADBEEF"), 5);
        assert_eq!(get_partition_id(07, "00000000-0606-0707-ACDC-DEADDEADBEEF"), 6);

        assert_eq!(get_partition_id(08, "00000000-0000-0808-8F92-DEADDEADBEEF"), 0);
        assert_eq!(get_partition_id(08, "00000000-0101-0808-8EF0-DEADDEADBEEF"), 1);
        assert_eq!(get_partition_id(08, "00000000-0202-0808-97A4-DEADDEADBEEF"), 2);
        assert_eq!(get_partition_id(08, "00000000-0303-0808-B4C9-DEADDEADBEEF"), 3);
        assert_eq!(get_partition_id(08, "00000000-0404-0808-9869-DEADDEADBEEF"), 4);
        assert_eq!(get_partition_id(08, "00000000-0505-0808-9D54-DEADDEADBEEF"), 5);
        assert_eq!(get_partition_id(08, "00000000-0606-0808-83C4-DEADDEADBEEF"), 6);
        assert_eq!(get_partition_id(08, "00000000-0707-0808-9258-DEADDEADBEEF"), 7);

        assert_eq!(get_partition_id(09, "00000000-0000-0909-9916-DEADDEADBEEF"), 0);
        assert_eq!(get_partition_id(09, "00000000-0101-0909-95BC-DEADDEADBEEF"), 1);
        assert_eq!(get_partition_id(09, "00000000-0202-0909-9327-DEADDEADBEEF"), 2);
        assert_eq!(get_partition_id(09, "00000000-0303-0909-8ABD-DEADDEADBEEF"), 3);
        assert_eq!(get_partition_id(09, "00000000-0404-0909-AAA1-DEADDEADBEEF"), 4);
        assert_eq!(get_partition_id(09, "00000000-0505-0909-BA3F-DEADDEADBEEF"), 5);
        assert_eq!(get_partition_id(09, "00000000-0606-0909-941D-DEADDEADBEEF"), 6);
        assert_eq!(get_partition_id(09, "00000000-0707-0909-B938-DEADDEADBEEF"), 7);
        assert_eq!(get_partition_id(09, "00000000-0808-0909-A60F-DEADDEADBEEF"), 8);

        assert_eq!(get_partition_id(10, "00000000-0000-1010-AC89-DEADDEADBEEF"), 0);
        assert_eq!(get_partition_id(10, "00000000-0101-1010-B158-DEADDEADBEEF"), 1);
        assert_eq!(get_partition_id(10, "00000000-0202-1010-B240-DEADDEADBEEF"), 2);
        assert_eq!(get_partition_id(10, "00000000-0303-1010-8F18-DEADDEADBEEF"), 3);
        assert_eq!(get_partition_id(10, "00000000-0404-1010-9BAD-DEADDEADBEEF"), 4);
        assert_eq!(get_partition_id(10, "00000000-0505-1010-88C4-DEADDEADBEEF"), 5);
        assert_eq!(get_partition_id(10, "00000000-0606-1010-9D4D-DEADDEADBEEF"), 6);
        assert_eq!(get_partition_id(10, "00000000-0707-1010-89A3-DEADDEADBEEF"), 7);
        assert_eq!(get_partition_id(10, "00000000-0808-1010-92FB-DEADDEADBEEF"), 8);
        assert_eq!(get_partition_id(10, "00000000-0909-1010-9D92-DEADDEADBEEF"), 9);

        assert_eq!(get_partition_id(11, "00000000-0000-1111-A14E-DEADDEADBEEF"), 0);
        assert_eq!(get_partition_id(11, "00000000-0101-1111-8804-DEADDEADBEEF"), 1);
        assert_eq!(get_partition_id(11, "00000000-0202-1111-805B-DEADDEADBEEF"), 2);
        assert_eq!(get_partition_id(11, "00000000-0303-1111-96CF-DEADDEADBEEF"), 3);
        assert_eq!(get_partition_id(11, "00000000-0404-1111-B8A6-DEADDEADBEEF"), 4);
        assert_eq!(get_partition_id(11, "00000000-0505-1111-B0B7-DEADDEADBEEF"), 5);
        assert_eq!(get_partition_id(11, "00000000-0606-1111-9ECC-DEADDEADBEEF"), 6);
        assert_eq!(get_partition_id(11, "00000000-0707-1111-9FE5-DEADDEADBEEF"), 7);
        assert_eq!(get_partition_id(11, "00000000-0808-1111-B639-DEADDEADBEEF"), 8);
        assert_eq!(get_partition_id(11, "00000000-0909-1111-B69A-DEADDEADBEEF"), 9);
        assert_eq!(get_partition_id(11, "00000000-1010-1111-8008-DEADDEADBEEF"), 10);

        assert_eq!(get_partition_id(12, "00000000-0000-1212-9947-DEADDEADBEEF"), 0);
        assert_eq!(get_partition_id(12, "00000000-0101-1212-8E5F-DEADDEADBEEF"), 1);
        assert_eq!(get_partition_id(12, "00000000-0202-1212-AA3B-DEADDEADBEEF"), 2);
        assert_eq!(get_partition_id(12, "00000000-0303-1212-96C2-DEADDEADBEEF"), 3);
        assert_eq!(get_partition_id(12, "00000000-0404-1212-A35C-DEADDEADBEEF"), 4);
        assert_eq!(get_partition_id(12, "00000000-0505-1212-8B18-DEADDEADBEEF"), 5);
        assert_eq!(get_partition_id(12, "00000000-0606-1212-9FF6-DEADDEADBEEF"), 6);
        assert_eq!(get_partition_id(12, "00000000-0707-1212-B8AF-DEADDEADBEEF"), 7);
        assert_eq!(get_partition_id(12, "00000000-0808-1212-9578-DEADDEADBEEF"), 8);
        assert_eq!(get_partition_id(12, "00000000-0909-1212-BDAB-DEADDEADBEEF"), 9);
        assert_eq!(get_partition_id(12, "00000000-1010-1212-AF3A-DEADDEADBEEF"), 10);
        assert_eq!(get_partition_id(12, "00000000-1111-1212-BB13-DEADDEADBEEF"), 11);

        assert_eq!(get_partition_id(13, "00000000-0000-1313-A322-DEADDEADBEEF"), 0);
        assert_eq!(get_partition_id(13, "00000000-0101-1313-BF09-DEADDEADBEEF"), 1);
        assert_eq!(get_partition_id(13, "00000000-0202-1313-AC06-DEADDEADBEEF"), 2);
        assert_eq!(get_partition_id(13, "00000000-0303-1313-86D3-DEADDEADBEEF"), 3);
        assert_eq!(get_partition_id(13, "00000000-0404-1313-967B-DEADDEADBEEF"), 4);
        assert_eq!(get_partition_id(13, "00000000-0505-1313-821A-DEADDEADBEEF"), 5);
        assert_eq!(get_partition_id(13, "00000000-0606-1313-85E6-DEADDEADBEEF"), 6);
        assert_eq!(get_partition_id(13, "00000000-0707-1313-9722-DEADDEADBEEF"), 7);
        assert_eq!(get_partition_id(13, "00000000-0808-1313-A82B-DEADDEADBEEF"), 8);
        assert_eq!(get_partition_id(13, "00000000-0909-1313-B174-DEADDEADBEEF"), 9);
        assert_eq!(get_partition_id(13, "00000000-1010-1313-AC35-DEADDEADBEEF"), 10);
        assert_eq!(get_partition_id(13, "00000000-1111-1313-8719-DEADDEADBEEF"), 11);
        assert_eq!(get_partition_id(13, "00000000-1212-1313-ACEE-DEADDEADBEEF"), 12);

        assert_eq!(get_partition_id(14, "00000000-0000-1414-A81F-DEADDEADBEEF"), 0);
        assert_eq!(get_partition_id(14, "00000000-0101-1414-B539-DEADDEADBEEF"), 1);
        assert_eq!(get_partition_id(14, "00000000-0202-1414-AB90-DEADDEADBEEF"), 2);
        assert_eq!(get_partition_id(14, "00000000-0303-1414-98EA-DEADDEADBEEF"), 3);
        assert_eq!(get_partition_id(14, "00000000-0404-1414-A27D-DEADDEADBEEF"), 4);
        assert_eq!(get_partition_id(14, "00000000-0505-1414-BC2E-DEADDEADBEEF"), 5);
        assert_eq!(get_partition_id(14, "00000000-0606-1414-ABC7-DEADDEADBEEF"), 6);
        assert_eq!(get_partition_id(14, "00000000-0707-1414-8D6F-DEADDEADBEEF"), 7);
        assert_eq!(get_partition_id(14, "00000000-0808-1414-A254-DEADDEADBEEF"), 8);
        assert_eq!(get_partition_id(14, "00000000-0909-1414-B4F0-DEADDEADBEEF"), 9);
        assert_eq!(get_partition_id(14, "00000000-1010-1414-84C6-DEADDEADBEEF"), 10);
        assert_eq!(get_partition_id(14, "00000000-1111-1414-964B-DEADDEADBEEF"), 11);
        assert_eq!(get_partition_id(14, "00000000-1212-1414-8A62-DEADDEADBEEF"), 12);
        assert_eq!(get_partition_id(14, "00000000-1313-1414-975D-DEADDEADBEEF"), 13);

        assert_eq!(get_partition_id(15, "00000000-0000-1515-AE2C-DEADDEADBEEF"), 0);
        assert_eq!(get_partition_id(15, "00000000-0101-1515-A232-DEADDEADBEEF"), 1);
        assert_eq!(get_partition_id(15, "00000000-0202-1515-8212-DEADDEADBEEF"), 2);
        assert_eq!(get_partition_id(15, "00000000-0303-1515-B1B3-DEADDEADBEEF"), 3);
        assert_eq!(get_partition_id(15, "00000000-0404-1515-A791-DEADDEADBEEF"), 4);
        assert_eq!(get_partition_id(15, "00000000-0505-1515-92C3-DEADDEADBEEF"), 5);
        assert_eq!(get_partition_id(15, "00000000-0606-1515-9A88-DEADDEADBEEF"), 6);
        assert_eq!(get_partition_id(15, "00000000-0707-1515-894D-DEADDEADBEEF"), 7);
        assert_eq!(get_partition_id(15, "00000000-0808-1515-9A62-DEADDEADBEEF"), 8);
        assert_eq!(get_partition_id(15, "00000000-0909-1515-9FD0-DEADDEADBEEF"), 9);
        assert_eq!(get_partition_id(15, "00000000-1010-1515-8979-DEADDEADBEEF"), 10);
        assert_eq!(get_partition_id(15, "00000000-1111-1515-97E0-DEADDEADBEEF"), 11);
        assert_eq!(get_partition_id(15, "00000000-1212-1515-AED2-DEADDEADBEEF"), 12);
        assert_eq!(get_partition_id(15, "00000000-1313-1515-882F-DEADDEADBEEF"), 13);
        assert_eq!(get_partition_id(15, "00000000-1414-1515-A897-DEADDEADBEEF"), 14);

        assert_eq!(get_partition_id(16, "00000000-0000-1616-AA5C-DEADDEADBEEF"), 0);
        assert_eq!(get_partition_id(16, "00000000-0101-1616-8430-DEADDEADBEEF"), 1);
        assert_eq!(get_partition_id(16, "00000000-0202-1616-A500-DEADDEADBEEF"), 2);
        assert_eq!(get_partition_id(16, "00000000-0303-1616-BB01-DEADDEADBEEF"), 3);
        assert_eq!(get_partition_id(16, "00000000-0404-1616-B663-DEADDEADBEEF"), 4);
        assert_eq!(get_partition_id(16, "00000000-0505-1616-8E56-DEADDEADBEEF"), 5);
        assert_eq!(get_partition_id(16, "00000000-0606-1616-8883-DEADDEADBEEF"), 6);
        assert_eq!(get_partition_id(16, "00000000-0707-1616-8DDF-DEADDEADBEEF"), 7);
        assert_eq!(get_partition_id(16, "00000000-0808-1616-8ADD-DEADDEADBEEF"), 8);
        assert_eq!(get_partition_id(16, "00000000-0909-1616-A1E7-DEADDEADBEEF"), 9);
        assert_eq!(get_partition_id(16, "00000000-1010-1616-A7A3-DEADDEADBEEF"), 10);
        assert_eq!(get_partition_id(16, "00000000-1111-1616-B54B-DEADDEADBEEF"), 11);
        assert_eq!(get_partition_id(16, "00000000-1212-1616-A5B8-DEADDEADBEEF"), 12);
        assert_eq!(get_partition_id(16, "00000000-1313-1616-A606-DEADDEADBEEF"), 13);
        assert_eq!(get_partition_id(16, "00000000-1414-1616-A611-DEADDEADBEEF"), 14);
        assert_eq!(get_partition_id(16, "00000000-1515-1616-B304-DEADDEADBEEF"), 15);

        assert_eq!(get_partition_id(17, "00000000-0000-1717-A681-DEADDEADBEEF"), 0);
        assert_eq!(get_partition_id(17, "00000000-0101-1717-8C8B-DEADDEADBEEF"), 1);
        assert_eq!(get_partition_id(17, "00000000-0202-1717-B993-DEADDEADBEEF"), 2);
        assert_eq!(get_partition_id(17, "00000000-0303-1717-A2E9-DEADDEADBEEF"), 3);
        assert_eq!(get_partition_id(17, "00000000-0404-1717-89F2-DEADDEADBEEF"), 4);
        assert_eq!(get_partition_id(17, "00000000-0505-1717-8D08-DEADDEADBEEF"), 5);
        assert_eq!(get_partition_id(17, "00000000-0606-1717-9755-DEADDEADBEEF"), 6);
        assert_eq!(get_partition_id(17, "00000000-0707-1717-B3E9-DEADDEADBEEF"), 7);
        assert_eq!(get_partition_id(17, "00000000-0808-1717-9E20-DEADDEADBEEF"), 8);
        assert_eq!(get_partition_id(17, "00000000-0909-1717-8E29-DEADDEADBEEF"), 9);
        assert_eq!(get_partition_id(17, "00000000-1010-1717-8EFA-DEADDEADBEEF"), 10);
        assert_eq!(get_partition_id(17, "00000000-1111-1717-A249-DEADDEADBEEF"), 11);
        assert_eq!(get_partition_id(17, "00000000-1212-1717-9B9E-DEADDEADBEEF"), 12);
        assert_eq!(get_partition_id(17, "00000000-1313-1717-98F2-DEADDEADBEEF"), 13);
        assert_eq!(get_partition_id(17, "00000000-1414-1717-B6D8-DEADDEADBEEF"), 14);
        assert_eq!(get_partition_id(17, "00000000-1515-1717-BD94-DEADDEADBEEF"), 15);
        assert_eq!(get_partition_id(17, "00000000-1616-1717-A03D-DEADDEADBEEF"), 16);

        assert_eq!(get_partition_id(18, "00000000-0000-1818-8CF4-DEADDEADBEEF"), 0);
        assert_eq!(get_partition_id(18, "00000000-0101-1818-BB09-DEADDEADBEEF"), 1);
        assert_eq!(get_partition_id(18, "00000000-0202-1818-9424-DEADDEADBEEF"), 2);
        assert_eq!(get_partition_id(18, "00000000-0303-1818-AE06-DEADDEADBEEF"), 3);
        assert_eq!(get_partition_id(18, "00000000-0404-1818-B718-DEADDEADBEEF"), 4);
        assert_eq!(get_partition_id(18, "00000000-0505-1818-945D-DEADDEADBEEF"), 5);
        assert_eq!(get_partition_id(18, "00000000-0606-1818-A453-DEADDEADBEEF"), 6);
        assert_eq!(get_partition_id(18, "00000000-0707-1818-B192-DEADDEADBEEF"), 7);
        assert_eq!(get_partition_id(18, "00000000-0808-1818-8F9F-DEADDEADBEEF"), 8);
        assert_eq!(get_partition_id(18, "00000000-0909-1818-87A6-DEADDEADBEEF"), 9);
        assert_eq!(get_partition_id(18, "00000000-1010-1818-8AC7-DEADDEADBEEF"), 10);
        assert_eq!(get_partition_id(18, "00000000-1111-1818-907E-DEADDEADBEEF"), 11);
        assert_eq!(get_partition_id(18, "00000000-1212-1818-A552-DEADDEADBEEF"), 12);
        assert_eq!(get_partition_id(18, "00000000-1313-1818-8746-DEADDEADBEEF"), 13);
        assert_eq!(get_partition_id(18, "00000000-1414-1818-A327-DEADDEADBEEF"), 14);
        assert_eq!(get_partition_id(18, "00000000-1515-1818-83D7-DEADDEADBEEF"), 15);
        assert_eq!(get_partition_id(18, "00000000-1616-1818-8066-DEADDEADBEEF"), 16);
        assert_eq!(get_partition_id(18, "00000000-1717-1818-A46C-DEADDEADBEEF"), 17);

        assert_eq!(get_partition_id(19, "00000000-0000-1919-8252-DEADDEADBEEF"), 0);
        assert_eq!(get_partition_id(19, "00000000-0101-1919-8F70-DEADDEADBEEF"), 1);
        assert_eq!(get_partition_id(19, "00000000-0202-1919-9334-DEADDEADBEEF"), 2);
        assert_eq!(get_partition_id(19, "00000000-0303-1919-BC4B-DEADDEADBEEF"), 3);
        assert_eq!(get_partition_id(19, "00000000-0404-1919-90C9-DEADDEADBEEF"), 4);
        assert_eq!(get_partition_id(19, "00000000-0505-1919-B476-DEADDEADBEEF"), 5);
        assert_eq!(get_partition_id(19, "00000000-0606-1919-9953-DEADDEADBEEF"), 6);
        assert_eq!(get_partition_id(19, "00000000-0707-1919-B47F-DEADDEADBEEF"), 7);
        assert_eq!(get_partition_id(19, "00000000-0808-1919-B153-DEADDEADBEEF"), 8);
        assert_eq!(get_partition_id(19, "00000000-0909-1919-916D-DEADDEADBEEF"), 9);
        assert_eq!(get_partition_id(19, "00000000-1010-1919-8EA4-DEADDEADBEEF"), 10);
        assert_eq!(get_partition_id(19, "00000000-1111-1919-A837-DEADDEADBEEF"), 11);
        assert_eq!(get_partition_id(19, "00000000-1212-1919-BA57-DEADDEADBEEF"), 12);
        assert_eq!(get_partition_id(19, "00000000-1313-1919-809C-DEADDEADBEEF"), 13);
        assert_eq!(get_partition_id(19, "00000000-1414-1919-93AC-DEADDEADBEEF"), 14);
        assert_eq!(get_partition_id(19, "00000000-1515-1919-9875-DEADDEADBEEF"), 15);
        assert_eq!(get_partition_id(19, "00000000-1616-1919-8A0F-DEADDEADBEEF"), 16);
        assert_eq!(get_partition_id(19, "00000000-1717-1919-BC66-DEADDEADBEEF"), 17);
        assert_eq!(get_partition_id(19, "00000000-1818-1919-B1C3-DEADDEADBEEF"), 18);

        assert_eq!(get_partition_id(20, "00000000-0000-2020-BCC9-DEADDEADBEEF"), 0);
        assert_eq!(get_partition_id(20, "00000000-0101-2020-9296-DEADDEADBEEF"), 1);
        assert_eq!(get_partition_id(20, "00000000-0202-2020-AC51-DEADDEADBEEF"), 2);
        assert_eq!(get_partition_id(20, "00000000-0303-2020-9F33-DEADDEADBEEF"), 3);
        assert_eq!(get_partition_id(20, "00000000-0404-2020-9CDD-DEADDEADBEEF"), 4);
        assert_eq!(get_partition_id(20, "00000000-0505-2020-80A0-DEADDEADBEEF"), 5);
        assert_eq!(get_partition_id(20, "00000000-0606-2020-A077-DEADDEADBEEF"), 6);
        assert_eq!(get_partition_id(20, "00000000-0707-2020-8993-DEADDEADBEEF"), 7);
        assert_eq!(get_partition_id(20, "00000000-0808-2020-A58E-DEADDEADBEEF"), 8);
        assert_eq!(get_partition_id(20, "00000000-0909-2020-B6C9-DEADDEADBEEF"), 9);
        assert_eq!(get_partition_id(20, "00000000-1010-2020-8F71-DEADDEADBEEF"), 10);
        assert_eq!(get_partition_id(20, "00000000-1111-2020-86B3-DEADDEADBEEF"), 11);
        assert_eq!(get_partition_id(20, "00000000-1212-2020-AF23-DEADDEADBEEF"), 12);
        assert_eq!(get_partition_id(20, "00000000-1313-2020-80B0-DEADDEADBEEF"), 13);
        assert_eq!(get_partition_id(20, "00000000-1414-2020-962D-DEADDEADBEEF"), 14);
        assert_eq!(get_partition_id(20, "00000000-1515-2020-8267-DEADDEADBEEF"), 15);
        assert_eq!(get_partition_id(20, "00000000-1616-2020-BD73-DEADDEADBEEF"), 16);
        assert_eq!(get_partition_id(20, "00000000-1717-2020-90CB-DEADDEADBEEF"), 17);
        assert_eq!(get_partition_id(20, "00000000-1818-2020-87AF-DEADDEADBEEF"), 18);
        assert_eq!(get_partition_id(20, "00000000-1919-2020-8E86-DEADDEADBEEF"), 19);

        assert_eq!(get_partition_id(21, "00000000-0000-2121-9204-DEADDEADBEEF"), 0);
        assert_eq!(get_partition_id(21, "00000000-0101-2121-9378-DEADDEADBEEF"), 1);
        assert_eq!(get_partition_id(21, "00000000-0202-2121-A241-DEADDEADBEEF"), 2);
        assert_eq!(get_partition_id(21, "00000000-0303-2121-AE2B-DEADDEADBEEF"), 3);
        assert_eq!(get_partition_id(21, "00000000-0404-2121-B6CB-DEADDEADBEEF"), 4);
        assert_eq!(get_partition_id(21, "00000000-0505-2121-A7A4-DEADDEADBEEF"), 5);
        assert_eq!(get_partition_id(21, "00000000-0606-2121-BB58-DEADDEADBEEF"), 6);
        assert_eq!(get_partition_id(21, "00000000-0707-2121-B413-DEADDEADBEEF"), 7);
        assert_eq!(get_partition_id(21, "00000000-0808-2121-8E00-DEADDEADBEEF"), 8);
        assert_eq!(get_partition_id(21, "00000000-0909-2121-900A-DEADDEADBEEF"), 9);
        assert_eq!(get_partition_id(21, "00000000-1010-2121-9617-DEADDEADBEEF"), 10);
        assert_eq!(get_partition_id(21, "00000000-1111-2121-BFCC-DEADDEADBEEF"), 11);
        assert_eq!(get_partition_id(21, "00000000-1212-2121-B104-DEADDEADBEEF"), 12);
        assert_eq!(get_partition_id(21, "00000000-1313-2121-9C4C-DEADDEADBEEF"), 13);
        assert_eq!(get_partition_id(21, "00000000-1414-2121-97BE-DEADDEADBEEF"), 14);
        assert_eq!(get_partition_id(21, "00000000-1515-2121-A10D-DEADDEADBEEF"), 15);
        assert_eq!(get_partition_id(21, "00000000-1616-2121-9ACC-DEADDEADBEEF"), 16);
        assert_eq!(get_partition_id(21, "00000000-1717-2121-8C64-DEADDEADBEEF"), 17);
        assert_eq!(get_partition_id(21, "00000000-1818-2121-91CE-DEADDEADBEEF"), 18);
        assert_eq!(get_partition_id(21, "00000000-1919-2121-BC00-DEADDEADBEEF"), 19);
        assert_eq!(get_partition_id(21, "00000000-2020-2121-8EAF-DEADDEADBEEF"), 20);

        assert_eq!(get_partition_id(22, "00000000-0000-2222-8D69-DEADDEADBEEF"), 0);
        assert_eq!(get_partition_id(22, "00000000-0101-2222-8F68-DEADDEADBEEF"), 1);
        assert_eq!(get_partition_id(22, "00000000-0202-2222-815A-DEADDEADBEEF"), 2);
        assert_eq!(get_partition_id(22, "00000000-0303-2222-9447-DEADDEADBEEF"), 3);
        assert_eq!(get_partition_id(22, "00000000-0404-2222-9A14-DEADDEADBEEF"), 4);
        assert_eq!(get_partition_id(22, "00000000-0505-2222-8FD5-DEADDEADBEEF"), 5);
        assert_eq!(get_partition_id(22, "00000000-0606-2222-9715-DEADDEADBEEF"), 6);
        assert_eq!(get_partition_id(22, "00000000-0707-2222-A243-DEADDEADBEEF"), 7);
        assert_eq!(get_partition_id(22, "00000000-0808-2222-B50C-DEADDEADBEEF"), 8);
        assert_eq!(get_partition_id(22, "00000000-0909-2222-B703-DEADDEADBEEF"), 9);
        assert_eq!(get_partition_id(22, "00000000-1010-2222-97A7-DEADDEADBEEF"), 10);
        assert_eq!(get_partition_id(22, "00000000-1111-2222-8B4A-DEADDEADBEEF"), 11);
        assert_eq!(get_partition_id(22, "00000000-1212-2222-A3E7-DEADDEADBEEF"), 12);
        assert_eq!(get_partition_id(22, "00000000-1313-2222-BC1F-DEADDEADBEEF"), 13);
        assert_eq!(get_partition_id(22, "00000000-1414-2222-AD73-DEADDEADBEEF"), 14);
        assert_eq!(get_partition_id(22, "00000000-1515-2222-974C-DEADDEADBEEF"), 15);
        assert_eq!(get_partition_id(22, "00000000-1616-2222-8A99-DEADDEADBEEF"), 16);
        assert_eq!(get_partition_id(22, "00000000-1717-2222-A9A2-DEADDEADBEEF"), 17);
        assert_eq!(get_partition_id(22, "00000000-1818-2222-B9F7-DEADDEADBEEF"), 18);
        assert_eq!(get_partition_id(22, "00000000-1919-2222-A311-DEADDEADBEEF"), 19);
        assert_eq!(get_partition_id(22, "00000000-2020-2222-89FA-DEADDEADBEEF"), 20);
        assert_eq!(get_partition_id(22, "00000000-2121-2222-8BAF-DEADDEADBEEF"), 21);

        assert_eq!(get_partition_id(23, "00000000-0000-2323-921A-DEADDEADBEEF"), 0);
        assert_eq!(get_partition_id(23, "00000000-0101-2323-B848-DEADDEADBEEF"), 1);
        assert_eq!(get_partition_id(23, "00000000-0202-2323-A1AA-DEADDEADBEEF"), 2);
        assert_eq!(get_partition_id(23, "00000000-0303-2323-A42B-DEADDEADBEEF"), 3);
        assert_eq!(get_partition_id(23, "00000000-0404-2323-893D-DEADDEADBEEF"), 4);
        assert_eq!(get_partition_id(23, "00000000-0505-2323-A2BF-DEADDEADBEEF"), 5);
        assert_eq!(get_partition_id(23, "00000000-0606-2323-8200-DEADDEADBEEF"), 6);
        assert_eq!(get_partition_id(23, "00000000-0707-2323-8F17-DEADDEADBEEF"), 7);
        assert_eq!(get_partition_id(23, "00000000-0808-2323-A650-DEADDEADBEEF"), 8);
        assert_eq!(get_partition_id(23, "00000000-0909-2323-840B-DEADDEADBEEF"), 9);
        assert_eq!(get_partition_id(23, "00000000-1010-2323-9D7D-DEADDEADBEEF"), 10);
        assert_eq!(get_partition_id(23, "00000000-1111-2323-BE8B-DEADDEADBEEF"), 11);
        assert_eq!(get_partition_id(23, "00000000-1212-2323-BDBE-DEADDEADBEEF"), 12);
        assert_eq!(get_partition_id(23, "00000000-1313-2323-B930-DEADDEADBEEF"), 13);
        assert_eq!(get_partition_id(23, "00000000-1414-2323-9317-DEADDEADBEEF"), 14);
        assert_eq!(get_partition_id(23, "00000000-1515-2323-8586-DEADDEADBEEF"), 15);
        assert_eq!(get_partition_id(23, "00000000-1616-2323-9FE0-DEADDEADBEEF"), 16);
        assert_eq!(get_partition_id(23, "00000000-1717-2323-8AE4-DEADDEADBEEF"), 17);
        assert_eq!(get_partition_id(23, "00000000-1818-2323-A41C-DEADDEADBEEF"), 18);
        assert_eq!(get_partition_id(23, "00000000-1919-2323-A495-DEADDEADBEEF"), 19);
        assert_eq!(get_partition_id(23, "00000000-2020-2323-A0D1-DEADDEADBEEF"), 20);
        assert_eq!(get_partition_id(23, "00000000-2121-2323-AABF-DEADDEADBEEF"), 21);
        assert_eq!(get_partition_id(23, "00000000-2222-2323-9892-DEADDEADBEEF"), 22);

        assert_eq!(get_partition_id(24, "00000000-0000-2424-BEC0-DEADDEADBEEF"), 0);
        assert_eq!(get_partition_id(24, "00000000-0101-2424-A568-DEADDEADBEEF"), 1);
        assert_eq!(get_partition_id(24, "00000000-0202-2424-93DC-DEADDEADBEEF"), 2);
        assert_eq!(get_partition_id(24, "00000000-0303-2424-AC6C-DEADDEADBEEF"), 3);
        assert_eq!(get_partition_id(24, "00000000-0404-2424-9CF6-DEADDEADBEEF"), 4);
        assert_eq!(get_partition_id(24, "00000000-0505-2424-BC3F-DEADDEADBEEF"), 5);
        assert_eq!(get_partition_id(24, "00000000-0606-2424-9D82-DEADDEADBEEF"), 6);
        assert_eq!(get_partition_id(24, "00000000-0707-2424-B8F3-DEADDEADBEEF"), 7);
        assert_eq!(get_partition_id(24, "00000000-0808-2424-9FD0-DEADDEADBEEF"), 8);
        assert_eq!(get_partition_id(24, "00000000-0909-2424-82B8-DEADDEADBEEF"), 9);
        assert_eq!(get_partition_id(24, "00000000-1010-2424-B71E-DEADDEADBEEF"), 10);
        assert_eq!(get_partition_id(24, "00000000-1111-2424-8C00-DEADDEADBEEF"), 11);
        assert_eq!(get_partition_id(24, "00000000-1212-2424-8CFC-DEADDEADBEEF"), 12);
        assert_eq!(get_partition_id(24, "00000000-1313-2424-809A-DEADDEADBEEF"), 13);
        assert_eq!(get_partition_id(24, "00000000-1414-2424-BE71-DEADDEADBEEF"), 14);
        assert_eq!(get_partition_id(24, "00000000-1515-2424-8152-DEADDEADBEEF"), 15);
        assert_eq!(get_partition_id(24, "00000000-1616-2424-B9AD-DEADDEADBEEF"), 16);
        assert_eq!(get_partition_id(24, "00000000-1717-2424-B48D-DEADDEADBEEF"), 17);
        assert_eq!(get_partition_id(24, "00000000-1818-2424-9B8B-DEADDEADBEEF"), 18);
        assert_eq!(get_partition_id(24, "00000000-1919-2424-97CE-DEADDEADBEEF"), 19);
        assert_eq!(get_partition_id(24, "00000000-2020-2424-BC13-DEADDEADBEEF"), 20);
        assert_eq!(get_partition_id(24, "00000000-2121-2424-A615-DEADDEADBEEF"), 21);
        assert_eq!(get_partition_id(24, "00000000-2222-2424-8395-DEADDEADBEEF"), 22);
        assert_eq!(get_partition_id(24, "00000000-2323-2424-80E4-DEADDEADBEEF"), 23);

        assert_eq!(get_partition_id(25, "00000000-0000-2525-81ED-DEADDEADBEEF"), 0);
        assert_eq!(get_partition_id(25, "00000000-0101-2525-9D48-DEADDEADBEEF"), 1);
        assert_eq!(get_partition_id(25, "00000000-0202-2525-850A-DEADDEADBEEF"), 2);
        assert_eq!(get_partition_id(25, "00000000-0303-2525-896C-DEADDEADBEEF"), 3);
        assert_eq!(get_partition_id(25, "00000000-0404-2525-B29D-DEADDEADBEEF"), 4);
        assert_eq!(get_partition_id(25, "00000000-0505-2525-9510-DEADDEADBEEF"), 5);
        assert_eq!(get_partition_id(25, "00000000-0606-2525-B2C9-DEADDEADBEEF"), 6);
        assert_eq!(get_partition_id(25, "00000000-0707-2525-AC47-DEADDEADBEEF"), 7);
        assert_eq!(get_partition_id(25, "00000000-0808-2525-A2C1-DEADDEADBEEF"), 8);
        assert_eq!(get_partition_id(25, "00000000-0909-2525-B00E-DEADDEADBEEF"), 9);
        assert_eq!(get_partition_id(25, "00000000-1010-2525-8F68-DEADDEADBEEF"), 10);
        assert_eq!(get_partition_id(25, "00000000-1111-2525-9AF2-DEADDEADBEEF"), 11);
        assert_eq!(get_partition_id(25, "00000000-1212-2525-873E-DEADDEADBEEF"), 12);
        assert_eq!(get_partition_id(25, "00000000-1313-2525-8254-DEADDEADBEEF"), 13);
        assert_eq!(get_partition_id(25, "00000000-1414-2525-8F57-DEADDEADBEEF"), 14);
        assert_eq!(get_partition_id(25, "00000000-1515-2525-97D5-DEADDEADBEEF"), 15);
        assert_eq!(get_partition_id(25, "00000000-1616-2525-AFA0-DEADDEADBEEF"), 16);
        assert_eq!(get_partition_id(25, "00000000-1717-2525-BCD3-DEADDEADBEEF"), 17);
        assert_eq!(get_partition_id(25, "00000000-1818-2525-9D89-DEADDEADBEEF"), 18);
        assert_eq!(get_partition_id(25, "00000000-1919-2525-B63F-DEADDEADBEEF"), 19);
        assert_eq!(get_partition_id(25, "00000000-2020-2525-9D12-DEADDEADBEEF"), 20);
        assert_eq!(get_partition_id(25, "00000000-2121-2525-994E-DEADDEADBEEF"), 21);
        assert_eq!(get_partition_id(25, "00000000-2222-2525-AEE7-DEADDEADBEEF"), 22);
        assert_eq!(get_partition_id(25, "00000000-2323-2525-B39D-DEADDEADBEEF"), 23);
        assert_eq!(get_partition_id(25, "00000000-2424-2525-8DB3-DEADDEADBEEF"), 24);

        assert_eq!(get_partition_id(26, "00000000-0000-2626-8BF1-DEADDEADBEEF"), 0);
        assert_eq!(get_partition_id(26, "00000000-0101-2626-9ED7-DEADDEADBEEF"), 1);
        assert_eq!(get_partition_id(26, "00000000-0202-2626-BA76-DEADDEADBEEF"), 2);
        assert_eq!(get_partition_id(26, "00000000-0303-2626-B451-DEADDEADBEEF"), 3);
        assert_eq!(get_partition_id(26, "00000000-0404-2626-9E89-DEADDEADBEEF"), 4);
        assert_eq!(get_partition_id(26, "00000000-0505-2626-B8E6-DEADDEADBEEF"), 5);
        assert_eq!(get_partition_id(26, "00000000-0606-2626-8DE6-DEADDEADBEEF"), 6);
        assert_eq!(get_partition_id(26, "00000000-0707-2626-9090-DEADDEADBEEF"), 7);
        assert_eq!(get_partition_id(26, "00000000-0808-2626-86B9-DEADDEADBEEF"), 8);
        assert_eq!(get_partition_id(26, "00000000-0909-2626-BB26-DEADDEADBEEF"), 9);
        assert_eq!(get_partition_id(26, "00000000-1010-2626-8CB3-DEADDEADBEEF"), 10);
        assert_eq!(get_partition_id(26, "00000000-1111-2626-B361-DEADDEADBEEF"), 11);
        assert_eq!(get_partition_id(26, "00000000-1212-2626-8587-DEADDEADBEEF"), 12);
        assert_eq!(get_partition_id(26, "00000000-1313-2626-ABB9-DEADDEADBEEF"), 13);
        assert_eq!(get_partition_id(26, "00000000-1414-2626-B203-DEADDEADBEEF"), 14);
        assert_eq!(get_partition_id(26, "00000000-1515-2626-B28F-DEADDEADBEEF"), 15);
        assert_eq!(get_partition_id(26, "00000000-1616-2626-9B5C-DEADDEADBEEF"), 16);
        assert_eq!(get_partition_id(26, "00000000-1717-2626-AA6B-DEADDEADBEEF"), 17);
        assert_eq!(get_partition_id(26, "00000000-1818-2626-A98B-DEADDEADBEEF"), 18);
        assert_eq!(get_partition_id(26, "00000000-1919-2626-9A98-DEADDEADBEEF"), 19);
        assert_eq!(get_partition_id(26, "00000000-2020-2626-B783-DEADDEADBEEF"), 20);
        assert_eq!(get_partition_id(26, "00000000-2121-2626-A022-DEADDEADBEEF"), 21);
        assert_eq!(get_partition_id(26, "00000000-2222-2626-AFF3-DEADDEADBEEF"), 22);
        assert_eq!(get_partition_id(26, "00000000-2323-2626-A446-DEADDEADBEEF"), 23);
        assert_eq!(get_partition_id(26, "00000000-2424-2626-A753-DEADDEADBEEF"), 24);
        assert_eq!(get_partition_id(26, "00000000-2525-2626-9791-DEADDEADBEEF"), 25);

        assert_eq!(get_partition_id(27, "00000000-0000-2727-93F7-DEADDEADBEEF"), 0);
        assert_eq!(get_partition_id(27, "00000000-0101-2727-8F4E-DEADDEADBEEF"), 1);
        assert_eq!(get_partition_id(27, "00000000-0202-2727-B1EF-DEADDEADBEEF"), 2);
        assert_eq!(get_partition_id(27, "00000000-0303-2727-A285-DEADDEADBEEF"), 3);
        assert_eq!(get_partition_id(27, "00000000-0404-2727-8AB2-DEADDEADBEEF"), 4);
        assert_eq!(get_partition_id(27, "00000000-0505-2727-8FFA-DEADDEADBEEF"), 5);
        assert_eq!(get_partition_id(27, "00000000-0606-2727-9643-DEADDEADBEEF"), 6);
        assert_eq!(get_partition_id(27, "00000000-0707-2727-A3AD-DEADDEADBEEF"), 7);
        assert_eq!(get_partition_id(27, "00000000-0808-2727-ACB6-DEADDEADBEEF"), 8);
        assert_eq!(get_partition_id(27, "00000000-0909-2727-B6F6-DEADDEADBEEF"), 9);
        assert_eq!(get_partition_id(27, "00000000-1010-2727-9A52-DEADDEADBEEF"), 10);
        assert_eq!(get_partition_id(27, "00000000-1111-2727-8245-DEADDEADBEEF"), 11);
        assert_eq!(get_partition_id(27, "00000000-1212-2727-B178-DEADDEADBEEF"), 12);
        assert_eq!(get_partition_id(27, "00000000-1313-2727-A9B7-DEADDEADBEEF"), 13);
        assert_eq!(get_partition_id(27, "00000000-1414-2727-BA3D-DEADDEADBEEF"), 14);
        assert_eq!(get_partition_id(27, "00000000-1515-2727-A2CC-DEADDEADBEEF"), 15);
        assert_eq!(get_partition_id(27, "00000000-1616-2727-B2DF-DEADDEADBEEF"), 16);
        assert_eq!(get_partition_id(27, "00000000-1717-2727-953D-DEADDEADBEEF"), 17);
        assert_eq!(get_partition_id(27, "00000000-1818-2727-BDA7-DEADDEADBEEF"), 18);
        assert_eq!(get_partition_id(27, "00000000-1919-2727-AA5F-DEADDEADBEEF"), 19);
        assert_eq!(get_partition_id(27, "00000000-2020-2727-9988-DEADDEADBEEF"), 20);
        assert_eq!(get_partition_id(27, "00000000-2121-2727-9497-DEADDEADBEEF"), 21);
        assert_eq!(get_partition_id(27, "00000000-2222-2727-9DE4-DEADDEADBEEF"), 22);
        assert_eq!(get_partition_id(27, "00000000-2323-2727-B7B4-DEADDEADBEEF"), 23);
        assert_eq!(get_partition_id(27, "00000000-2424-2727-B1C5-DEADDEADBEEF"), 24);
        assert_eq!(get_partition_id(27, "00000000-2525-2727-9A90-DEADDEADBEEF"), 25);
        assert_eq!(get_partition_id(27, "00000000-2626-2727-A0D7-DEADDEADBEEF"), 26);

        assert_eq!(get_partition_id(28, "00000000-0000-2828-A75D-DEADDEADBEEF"), 0);
        assert_eq!(get_partition_id(28, "00000000-0101-2828-8064-DEADDEADBEEF"), 1);
        assert_eq!(get_partition_id(28, "00000000-0202-2828-A21B-DEADDEADBEEF"), 2);
        assert_eq!(get_partition_id(28, "00000000-0303-2828-80F1-DEADDEADBEEF"), 3);
        assert_eq!(get_partition_id(28, "00000000-0404-2828-B0DB-DEADDEADBEEF"), 4);
        assert_eq!(get_partition_id(28, "00000000-0505-2828-8D4B-DEADDEADBEEF"), 5);
        assert_eq!(get_partition_id(28, "00000000-0606-2828-A581-DEADDEADBEEF"), 6);
        assert_eq!(get_partition_id(28, "00000000-0707-2828-8F15-DEADDEADBEEF"), 7);
        assert_eq!(get_partition_id(28, "00000000-0808-2828-940D-DEADDEADBEEF"), 8);
        assert_eq!(get_partition_id(28, "00000000-0909-2828-9F49-DEADDEADBEEF"), 9);
        assert_eq!(get_partition_id(28, "00000000-1010-2828-A359-DEADDEADBEEF"), 10);
        assert_eq!(get_partition_id(28, "00000000-1111-2828-ACF3-DEADDEADBEEF"), 11);
        assert_eq!(get_partition_id(28, "00000000-1212-2828-908F-DEADDEADBEEF"), 12);
        assert_eq!(get_partition_id(28, "00000000-1313-2828-BD8B-DEADDEADBEEF"), 13);
        assert_eq!(get_partition_id(28, "00000000-1414-2828-ADE1-DEADDEADBEEF"), 14);
        assert_eq!(get_partition_id(28, "00000000-1515-2828-99BB-DEADDEADBEEF"), 15);
        assert_eq!(get_partition_id(28, "00000000-1616-2828-A46E-DEADDEADBEEF"), 16);
        assert_eq!(get_partition_id(28, "00000000-1717-2828-A14B-DEADDEADBEEF"), 17);
        assert_eq!(get_partition_id(28, "00000000-1818-2828-8165-DEADDEADBEEF"), 18);
        assert_eq!(get_partition_id(28, "00000000-1919-2828-B13B-DEADDEADBEEF"), 19);
        assert_eq!(get_partition_id(28, "00000000-2020-2828-98EA-DEADDEADBEEF"), 20);
        assert_eq!(get_partition_id(28, "00000000-2121-2828-8C66-DEADDEADBEEF"), 21);
        assert_eq!(get_partition_id(28, "00000000-2222-2828-B4DB-DEADDEADBEEF"), 22);
        assert_eq!(get_partition_id(28, "00000000-2323-2828-B227-DEADDEADBEEF"), 23);
        assert_eq!(get_partition_id(28, "00000000-2424-2828-9B50-DEADDEADBEEF"), 24);
        assert_eq!(get_partition_id(28, "00000000-2525-2828-A85B-DEADDEADBEEF"), 25);
        assert_eq!(get_partition_id(28, "00000000-2626-2828-93B1-DEADDEADBEEF"), 26);
        assert_eq!(get_partition_id(28, "00000000-2727-2828-A0A6-DEADDEADBEEF"), 27);

        assert_eq!(get_partition_id(29, "00000000-0000-2929-B43F-DEADDEADBEEF"), 0);
        assert_eq!(get_partition_id(29, "00000000-0101-2929-A9E3-DEADDEADBEEF"), 1);
        assert_eq!(get_partition_id(29, "00000000-0202-2929-BD43-DEADDEADBEEF"), 2);
        assert_eq!(get_partition_id(29, "00000000-0303-2929-AF1D-DEADDEADBEEF"), 3);
        assert_eq!(get_partition_id(29, "00000000-0404-2929-94CD-DEADDEADBEEF"), 4);
        assert_eq!(get_partition_id(29, "00000000-0505-2929-8AFE-DEADDEADBEEF"), 5);
        assert_eq!(get_partition_id(29, "00000000-0606-2929-9445-DEADDEADBEEF"), 6);
        assert_eq!(get_partition_id(29, "00000000-0707-2929-AD30-DEADDEADBEEF"), 7);
        assert_eq!(get_partition_id(29, "00000000-0808-2929-B995-DEADDEADBEEF"), 8);
        assert_eq!(get_partition_id(29, "00000000-0909-2929-BD31-DEADDEADBEEF"), 9);
        assert_eq!(get_partition_id(29, "00000000-1010-2929-A8D7-DEADDEADBEEF"), 10);
        assert_eq!(get_partition_id(29, "00000000-1111-2929-AE06-DEADDEADBEEF"), 11);
        assert_eq!(get_partition_id(29, "00000000-1212-2929-9C93-DEADDEADBEEF"), 12);
        assert_eq!(get_partition_id(29, "00000000-1313-2929-B9E7-DEADDEADBEEF"), 13);
        assert_eq!(get_partition_id(29, "00000000-1414-2929-ABF0-DEADDEADBEEF"), 14);
        assert_eq!(get_partition_id(29, "00000000-1515-2929-B83D-DEADDEADBEEF"), 15);
        assert_eq!(get_partition_id(29, "00000000-1616-2929-A25A-DEADDEADBEEF"), 16);
        assert_eq!(get_partition_id(29, "00000000-1717-2929-9CCC-DEADDEADBEEF"), 17);
        assert_eq!(get_partition_id(29, "00000000-1818-2929-886C-DEADDEADBEEF"), 18);
        assert_eq!(get_partition_id(29, "00000000-1919-2929-B785-DEADDEADBEEF"), 19);
        assert_eq!(get_partition_id(29, "00000000-2020-2929-8460-DEADDEADBEEF"), 20);
        assert_eq!(get_partition_id(29, "00000000-2121-2929-8321-DEADDEADBEEF"), 21);
        assert_eq!(get_partition_id(29, "00000000-2222-2929-AC72-DEADDEADBEEF"), 22);
        assert_eq!(get_partition_id(29, "00000000-2323-2929-A47B-DEADDEADBEEF"), 23);
        assert_eq!(get_partition_id(29, "00000000-2424-2929-92CB-DEADDEADBEEF"), 24);
        assert_eq!(get_partition_id(29, "00000000-2525-2929-A0D3-DEADDEADBEEF"), 25);
        assert_eq!(get_partition_id(29, "00000000-2626-2929-947E-DEADDEADBEEF"), 26);
        assert_eq!(get_partition_id(29, "00000000-2727-2929-BF0A-DEADDEADBEEF"), 27);
        assert_eq!(get_partition_id(29, "00000000-2828-2929-BD39-DEADDEADBEEF"), 28);

        assert_eq!(get_partition_id(30, "00000000-0000-3030-BFAD-DEADDEADBEEF"), 0);
        assert_eq!(get_partition_id(30, "00000000-0101-3030-BC72-DEADDEADBEEF"), 1);
        assert_eq!(get_partition_id(30, "00000000-0202-3030-912E-DEADDEADBEEF"), 2);
        assert_eq!(get_partition_id(30, "00000000-0303-3030-A220-DEADDEADBEEF"), 3);
        assert_eq!(get_partition_id(30, "00000000-0404-3030-A7F1-DEADDEADBEEF"), 4);
        assert_eq!(get_partition_id(30, "00000000-0505-3030-A281-DEADDEADBEEF"), 5);
        assert_eq!(get_partition_id(30, "00000000-0606-3030-BBF2-DEADDEADBEEF"), 6);
        assert_eq!(get_partition_id(30, "00000000-0707-3030-9CDD-DEADDEADBEEF"), 7);
        assert_eq!(get_partition_id(30, "00000000-0808-3030-8A68-DEADDEADBEEF"), 8);
        assert_eq!(get_partition_id(30, "00000000-0909-3030-9DBB-DEADDEADBEEF"), 9);
        assert_eq!(get_partition_id(30, "00000000-1010-3030-906F-DEADDEADBEEF"), 10);
        assert_eq!(get_partition_id(30, "00000000-1111-3030-9082-DEADDEADBEEF"), 11);
        assert_eq!(get_partition_id(30, "00000000-1212-3030-917D-DEADDEADBEEF"), 12);
        assert_eq!(get_partition_id(30, "00000000-1313-3030-93A2-DEADDEADBEEF"), 13);
        assert_eq!(get_partition_id(30, "00000000-1414-3030-968B-DEADDEADBEEF"), 14);
        assert_eq!(get_partition_id(30, "00000000-1515-3030-BAF5-DEADDEADBEEF"), 15);
        assert_eq!(get_partition_id(30, "00000000-1616-3030-B048-DEADDEADBEEF"), 16);
        assert_eq!(get_partition_id(30, "00000000-1717-3030-89D8-DEADDEADBEEF"), 17);
        assert_eq!(get_partition_id(30, "00000000-1818-3030-B394-DEADDEADBEEF"), 18);
        assert_eq!(get_partition_id(30, "00000000-1919-3030-AF6B-DEADDEADBEEF"), 19);
        assert_eq!(get_partition_id(30, "00000000-2020-3030-AC9E-DEADDEADBEEF"), 20);
        assert_eq!(get_partition_id(30, "00000000-2121-3030-BD96-DEADDEADBEEF"), 21);
        assert_eq!(get_partition_id(30, "00000000-2222-3030-A464-DEADDEADBEEF"), 22);
        assert_eq!(get_partition_id(30, "00000000-2323-3030-A115-DEADDEADBEEF"), 23);
        assert_eq!(get_partition_id(30, "00000000-2424-3030-B735-DEADDEADBEEF"), 24);
        assert_eq!(get_partition_id(30, "00000000-2525-3030-A500-DEADDEADBEEF"), 25);
        assert_eq!(get_partition_id(30, "00000000-2626-3030-A972-DEADDEADBEEF"), 26);
        assert_eq!(get_partition_id(30, "00000000-2727-3030-BE84-DEADDEADBEEF"), 27);
        assert_eq!(get_partition_id(30, "00000000-2828-3030-8006-DEADDEADBEEF"), 28);
        assert_eq!(get_partition_id(30, "00000000-2929-3030-A617-DEADDEADBEEF"), 29);

        assert_eq!(get_partition_id(31, "00000000-0000-3131-A525-DEADDEADBEEF"), 0);
        assert_eq!(get_partition_id(31, "00000000-0101-3131-BCDE-DEADDEADBEEF"), 1);
        assert_eq!(get_partition_id(31, "00000000-0202-3131-8619-DEADDEADBEEF"), 2);
        assert_eq!(get_partition_id(31, "00000000-0303-3131-B99A-DEADDEADBEEF"), 3);
        assert_eq!(get_partition_id(31, "00000000-0404-3131-9050-DEADDEADBEEF"), 4);
        assert_eq!(get_partition_id(31, "00000000-0505-3131-8BAA-DEADDEADBEEF"), 5);
        assert_eq!(get_partition_id(31, "00000000-0606-3131-B242-DEADDEADBEEF"), 6);
        assert_eq!(get_partition_id(31, "00000000-0707-3131-82AE-DEADDEADBEEF"), 7);
        assert_eq!(get_partition_id(31, "00000000-0808-3131-8C86-DEADDEADBEEF"), 8);
        assert_eq!(get_partition_id(31, "00000000-0909-3131-A891-DEADDEADBEEF"), 9);
        assert_eq!(get_partition_id(31, "00000000-1010-3131-9A08-DEADDEADBEEF"), 10);
        assert_eq!(get_partition_id(31, "00000000-1111-3131-941B-DEADDEADBEEF"), 11);
        assert_eq!(get_partition_id(31, "00000000-1212-3131-962F-DEADDEADBEEF"), 12);
        assert_eq!(get_partition_id(31, "00000000-1313-3131-8B56-DEADDEADBEEF"), 13);
        assert_eq!(get_partition_id(31, "00000000-1414-3131-81A3-DEADDEADBEEF"), 14);
        assert_eq!(get_partition_id(31, "00000000-1515-3131-B9F5-DEADDEADBEEF"), 15);
        assert_eq!(get_partition_id(31, "00000000-1616-3131-8996-DEADDEADBEEF"), 16);
        assert_eq!(get_partition_id(31, "00000000-1717-3131-BE2A-DEADDEADBEEF"), 17);
        assert_eq!(get_partition_id(31, "00000000-1818-3131-B4B8-DEADDEADBEEF"), 18);
        assert_eq!(get_partition_id(31, "00000000-1919-3131-AA63-DEADDEADBEEF"), 19);
        assert_eq!(get_partition_id(31, "00000000-2020-3131-A74D-DEADDEADBEEF"), 20);
        assert_eq!(get_partition_id(31, "00000000-2121-3131-B14B-DEADDEADBEEF"), 21);
        assert_eq!(get_partition_id(31, "00000000-2222-3131-A2FA-DEADDEADBEEF"), 22);
        assert_eq!(get_partition_id(31, "00000000-2323-3131-A51B-DEADDEADBEEF"), 23);
        assert_eq!(get_partition_id(31, "00000000-2424-3131-A6BB-DEADDEADBEEF"), 24);
        assert_eq!(get_partition_id(31, "00000000-2525-3131-A393-DEADDEADBEEF"), 25);
        assert_eq!(get_partition_id(31, "00000000-2626-3131-AF58-DEADDEADBEEF"), 26);
        assert_eq!(get_partition_id(31, "00000000-2727-3131-83D8-DEADDEADBEEF"), 27);
        assert_eq!(get_partition_id(31, "00000000-2828-3131-90F5-DEADDEADBEEF"), 28);
        assert_eq!(get_partition_id(31, "00000000-2929-3131-B89F-DEADDEADBEEF"), 29);
        assert_eq!(get_partition_id(31, "00000000-3030-3131-A707-DEADDEADBEEF"), 30);

        assert_eq!(get_partition_id(32, "00000000-0000-3232-B086-DEADDEADBEEF"), 0);
        assert_eq!(get_partition_id(32, "00000000-0101-3232-BFAE-DEADDEADBEEF"), 1);
        assert_eq!(get_partition_id(32, "00000000-0202-3232-8D06-DEADDEADBEEF"), 2);
        assert_eq!(get_partition_id(32, "00000000-0303-3232-A424-DEADDEADBEEF"), 3);
        assert_eq!(get_partition_id(32, "00000000-0404-3232-A296-DEADDEADBEEF"), 4);
        assert_eq!(get_partition_id(32, "00000000-0505-3232-BE62-DEADDEADBEEF"), 5);
        assert_eq!(get_partition_id(32, "00000000-0606-3232-A3AB-DEADDEADBEEF"), 6);
        assert_eq!(get_partition_id(32, "00000000-0707-3232-8BCA-DEADDEADBEEF"), 7);
        assert_eq!(get_partition_id(32, "00000000-0808-3232-9228-DEADDEADBEEF"), 8);
        assert_eq!(get_partition_id(32, "00000000-0909-3232-A703-DEADDEADBEEF"), 9);
        assert_eq!(get_partition_id(32, "00000000-1010-3232-9E83-DEADDEADBEEF"), 10);
        assert_eq!(get_partition_id(32, "00000000-1111-3232-B904-DEADDEADBEEF"), 11);
        assert_eq!(get_partition_id(32, "00000000-1212-3232-8DAE-DEADDEADBEEF"), 12);
        assert_eq!(get_partition_id(32, "00000000-1313-3232-A2B4-DEADDEADBEEF"), 13);
        assert_eq!(get_partition_id(32, "00000000-1414-3232-9725-DEADDEADBEEF"), 14);
        assert_eq!(get_partition_id(32, "00000000-1515-3232-8029-DEADDEADBEEF"), 15);
        assert_eq!(get_partition_id(32, "00000000-1616-3232-BDF4-DEADDEADBEEF"), 16);
        assert_eq!(get_partition_id(32, "00000000-1717-3232-9073-DEADDEADBEEF"), 17);
        assert_eq!(get_partition_id(32, "00000000-1818-3232-AC8C-DEADDEADBEEF"), 18);
        assert_eq!(get_partition_id(32, "00000000-1919-3232-B968-DEADDEADBEEF"), 19);
        assert_eq!(get_partition_id(32, "00000000-2020-3232-B406-DEADDEADBEEF"), 20);
        assert_eq!(get_partition_id(32, "00000000-2121-3232-ABEA-DEADDEADBEEF"), 21);
        assert_eq!(get_partition_id(32, "00000000-2222-3232-8F73-DEADDEADBEEF"), 22);
        assert_eq!(get_partition_id(32, "00000000-2323-3232-884B-DEADDEADBEEF"), 23);
        assert_eq!(get_partition_id(32, "00000000-2424-3232-A0A5-DEADDEADBEEF"), 24);
        assert_eq!(get_partition_id(32, "00000000-2525-3232-B5FB-DEADDEADBEEF"), 25);
        assert_eq!(get_partition_id(32, "00000000-2626-3232-8640-DEADDEADBEEF"), 26);
        assert_eq!(get_partition_id(32, "00000000-2727-3232-8334-DEADDEADBEEF"), 27);
        assert_eq!(get_partition_id(32, "00000000-2828-3232-A80A-DEADDEADBEEF"), 28);
        assert_eq!(get_partition_id(32, "00000000-2929-3232-898B-DEADDEADBEEF"), 29);
        assert_eq!(get_partition_id(32, "00000000-3030-3232-B3BE-DEADDEADBEEF"), 30);
        assert_eq!(get_partition_id(32, "00000000-3131-3232-B974-DEADDEADBEEF"), 31);

        Ok(())
    }
}
