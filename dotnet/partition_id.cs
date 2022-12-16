namespace partition_id;
    
using System.Globalization;
using System.Text;

public static class Hash
{
    public static short DeterminePartitionId(this string partitionKey, short entityPartitionCount)
    {
        static IList<int> GetRanges(short rangeCount)
        {
            List<int> ranges = new(rangeCount); 
            
            int count = short.MaxValue;
            int partitionsPerRangeBase = (int)Math.Floor((decimal)count / (decimal)rangeCount);
            int remainingPartitions = count - (rangeCount * partitionsPerRangeBase);
            
            for (int i = 0, end = -1; i < rangeCount - 1; i++)
            {
                int partitiontPerRange =
                    i < remainingPartitions
                    ? partitionsPerRangeBase + 1
                    : partitionsPerRangeBase;

                end = (int)Math.Min(end + partitiontPerRange, count - 1);
                ranges.Add(end);
            }

            ranges.Add(count - 1);

            return ranges;
        }

        static short ToLogical(string partitionKey)
        {
            static (UInt32, UInt32, UInt64) hash(byte[] bytes, UInt32 pc = 0, UInt32 pb = 0)
            {
                static (uint, uint, ulong) combine(uint c, uint b)
                {
                    var r = (ulong)c + (((ulong)b) << 32);
                    return (c, b, r);
                }

                static void mix(ref uint a, ref uint b, ref uint c)
                {
                    a -= c; a ^= (c << 4) | (c >> 28); c += b;
                    b -= a; b ^= (a << 6) | (a >> 26); a += c;
                    c -= b; c ^= (b << 8) | (b >> 24); b += a;
                    a -= c; a ^= (c << 16) | (c >> 16); c += b;
                    b -= a; b ^= (a << 19) | (a >> 13); a += c;
                    c -= b; c ^= (b << 4) | (b >> 28); b += a;
                }

                static void final_mix(ref uint a, ref uint b, ref uint c)
                {
                    c ^= b; c -= (b << 14) | (b >> 18);
                    a ^= c; a -= (c << 11) | (c >> 21);
                    b ^= a; b -= (a << 25) | (a >> 7);
                    c ^= b; c -= (b << 16) | (b >> 16);
                    a ^= c; a -= (c << 4) | (c >> 28);
                    b ^= a; b -= (a << 14) | (a >> 18);
                    c ^= b; c -= (b << 24) | (b >> 8);
                }

                var initial = (uint)(0xdeadbeef + bytes.Length + pc);
                UInt32 a = initial;
                UInt32 b = initial;
                UInt32 c = initial;
                c += pb;

                int index = 0, size = bytes.Length;
                while (size > 12)
                {
                    a += BitConverter.ToUInt32(bytes, index);
                    b += BitConverter.ToUInt32(bytes, index + 4);
                    c += BitConverter.ToUInt32(bytes, index + 8);

                    mix(ref a, ref b, ref c);

                    index += 12;
                    size -= 12;
                }

                switch (size)
                {
                    case 12:
                        c += BitConverter.ToUInt32(bytes, index + 8);
                        b += BitConverter.ToUInt32(bytes, index + 4);
                        a += BitConverter.ToUInt32(bytes, index);
                        break;
                    case 11:
                        c += ((uint)bytes[index + 10]) << 16;
                        goto case 10;
                    case 10:
                        c += ((uint)bytes[index + 9]) << 8;
                        goto case 9;
                    case 9:
                        c += (uint)bytes[index + 8];
                        goto case 8;
                    case 8:
                        b += BitConverter.ToUInt32(bytes, index + 4);
                        a += BitConverter.ToUInt32(bytes, index);
                        break;
                    case 7:
                        b += ((uint)bytes[index + 6]) << 16;
                        goto case 6;
                    case 6:
                        b += ((uint)bytes[index + 5]) << 8;
                        goto case 5;
                    case 5:
                        b += (uint)bytes[index + 4];
                        goto case 4;
                    case 4:
                        a += BitConverter.ToUInt32(bytes, index);
                        break;
                    case 3:
                        a += ((uint)bytes[index + 2]) << 16;
                        goto case 2;
                    case 2:
                        a += ((uint)bytes[index + 1]) << 8;
                        goto case 1;
                    case 1:
                        a += (uint)bytes[index];
                        break;
                    case 0:
                        return combine(c, b);
                }

                final_mix(ref a, ref b, ref c);

                return combine(c, b);
            }

            if (partitionKey == null) { return 0; }

            (uint hash1, uint hash2, _) = hash(ASCIIEncoding.ASCII.GetBytes(partitionKey.ToUpper(CultureInfo.InvariantCulture)), pc: 0, pb: 0);

            return (short)Math.Abs((hash1 ^ hash2) % short.MaxValue);
        }
        
        static short ToPartitionId(IList<int> ranges, short partition)
        {
            var lower = 0;
            var upper = ranges.Count - 1;
            while (lower < upper)
            {
                int middle = (lower + upper) >> 1;

                if (partition > ranges[middle])
                {
                    lower = middle + 1;
                }
                else
                {
                    upper = middle;
                }
            }

            return (short) lower;
        }

        return ToPartitionId(
            ranges: GetRanges(entityPartitionCount), 
            partition: ToLogical(partitionKey));
    }
}

public class MyTester
{
    static string BruteForceGetFor(short desiredPartitionId, short partitionCount)
    {
        var desiredPrefix = $"00000000-{desiredPartitionId:00}{desiredPartitionId:00}-{partitionCount:00}{partitionCount:00}-";
        while (true)
        {
            var guid = Guid.NewGuid().ToString();
            var key = string.Concat(desiredPrefix, guid.AsSpan(19,4), "-DEADDEADBEEF");
            var partitionId = key.DeterminePartitionId(partitionCount);
            if (partitionId == desiredPartitionId)
            {
                return key.ToUpper();
            }
        }
    }

    static void BruteForce()
    {
        for (short partitionCount = 1; partitionCount <= 32; partitionCount++)
        {
            for (short desiredPartitionId = 0; desiredPartitionId < partitionCount; desiredPartitionId++)
            {
                Console.WriteLine($"({desiredPartitionId}, {partitionCount}, \"{BruteForceGetFor(desiredPartitionId, partitionCount)}\"),");
            }
            Console.WriteLine();
        }
    }

    static void TestCompliance()
    {
        var list = new[] {
            (0, 1, "00000000-0000-0101-9A83-DEADDEADBEEF"),

            (0, 2, "00000000-0000-0202-94F1-DEADDEADBEEF"),
            (1, 2, "00000000-0101-0202-B117-DEADDEADBEEF"),

            (0, 3, "00000000-0000-0303-8BF0-DEADDEADBEEF"),
            (1, 3, "00000000-0101-0303-BF1B-DEADDEADBEEF"),
            (2, 3, "00000000-0202-0303-8E30-DEADDEADBEEF"),

            (0, 4, "00000000-0000-0404-85C7-DEADDEADBEEF"),
            (1, 4, "00000000-0101-0404-AEB1-DEADDEADBEEF"),
            (2, 4, "00000000-0202-0404-B6DA-DEADDEADBEEF"),
            (3, 4, "00000000-0303-0404-8557-DEADDEADBEEF"),

            (0, 5, "00000000-0000-0505-A086-DEADDEADBEEF"),
            (1, 5, "00000000-0101-0505-8DAE-DEADDEADBEEF"),
            (2, 5, "00000000-0202-0505-950D-DEADDEADBEEF"),
            (3, 5, "00000000-0303-0505-98CA-DEADDEADBEEF"),
            (4, 5, "00000000-0404-0505-B4C9-DEADDEADBEEF"),

            (0, 6, "00000000-0000-0606-A73E-DEADDEADBEEF"),
            (1, 6, "00000000-0101-0606-8BBD-DEADDEADBEEF"),
            (2, 6, "00000000-0202-0606-A12E-DEADDEADBEEF"),
            (3, 6, "00000000-0303-0606-B935-DEADDEADBEEF"),
            (4, 6, "00000000-0404-0606-8D62-DEADDEADBEEF"),
            (5, 6, "00000000-0505-0606-AE21-DEADDEADBEEF"),

            (0, 7, "00000000-0000-0707-AF8B-DEADDEADBEEF"),
            (1, 7, "00000000-0101-0707-A48B-DEADDEADBEEF"),
            (2, 7, "00000000-0202-0707-B9EC-DEADDEADBEEF"),
            (3, 7, "00000000-0303-0707-961B-DEADDEADBEEF"),
            (4, 7, "00000000-0404-0707-8B09-DEADDEADBEEF"),
            (5, 7, "00000000-0505-0707-83B8-DEADDEADBEEF"),
            (6, 7, "00000000-0606-0707-ACDC-DEADDEADBEEF"),

            (0, 8, "00000000-0000-0808-8F92-DEADDEADBEEF"),
            (1, 8, "00000000-0101-0808-8EF0-DEADDEADBEEF"),
            (2, 8, "00000000-0202-0808-97A4-DEADDEADBEEF"),
            (3, 8, "00000000-0303-0808-B4C9-DEADDEADBEEF"),
            (4, 8, "00000000-0404-0808-9869-DEADDEADBEEF"),
            (5, 8, "00000000-0505-0808-9D54-DEADDEADBEEF"),
            (6, 8, "00000000-0606-0808-83C4-DEADDEADBEEF"),
            (7, 8, "00000000-0707-0808-9258-DEADDEADBEEF"),

            (0, 9, "00000000-0000-0909-9916-DEADDEADBEEF"),
            (1, 9, "00000000-0101-0909-95BC-DEADDEADBEEF"),
            (2, 9, "00000000-0202-0909-9327-DEADDEADBEEF"),
            (3, 9, "00000000-0303-0909-8ABD-DEADDEADBEEF"),
            (4, 9, "00000000-0404-0909-AAA1-DEADDEADBEEF"),
            (5, 9, "00000000-0505-0909-BA3F-DEADDEADBEEF"),
            (6, 9, "00000000-0606-0909-941D-DEADDEADBEEF"),
            (7, 9, "00000000-0707-0909-B938-DEADDEADBEEF"),
            (8, 9, "00000000-0808-0909-A60F-DEADDEADBEEF"),

            (0, 10, "00000000-0000-1010-AC89-DEADDEADBEEF"),
            (1, 10, "00000000-0101-1010-B158-DEADDEADBEEF"),
            (2, 10, "00000000-0202-1010-B240-DEADDEADBEEF"),
            (3, 10, "00000000-0303-1010-8F18-DEADDEADBEEF"),
            (4, 10, "00000000-0404-1010-9BAD-DEADDEADBEEF"),
            (5, 10, "00000000-0505-1010-88C4-DEADDEADBEEF"),
            (6, 10, "00000000-0606-1010-9D4D-DEADDEADBEEF"),
            (7, 10, "00000000-0707-1010-89A3-DEADDEADBEEF"),
            (8, 10, "00000000-0808-1010-92FB-DEADDEADBEEF"),
            (9, 10, "00000000-0909-1010-9D92-DEADDEADBEEF"),

            (0, 11, "00000000-0000-1111-A14E-DEADDEADBEEF"),
            (1, 11, "00000000-0101-1111-8804-DEADDEADBEEF"),
            (2, 11, "00000000-0202-1111-805B-DEADDEADBEEF"),
            (3, 11, "00000000-0303-1111-96CF-DEADDEADBEEF"),
            (4, 11, "00000000-0404-1111-B8A6-DEADDEADBEEF"),
            (5, 11, "00000000-0505-1111-B0B7-DEADDEADBEEF"),
            (6, 11, "00000000-0606-1111-9ECC-DEADDEADBEEF"),
            (7, 11, "00000000-0707-1111-9FE5-DEADDEADBEEF"),
            (8, 11, "00000000-0808-1111-B639-DEADDEADBEEF"),
            (9, 11, "00000000-0909-1111-B69A-DEADDEADBEEF"),
            (10, 11, "00000000-1010-1111-8008-DEADDEADBEEF"),

            (0, 12, "00000000-0000-1212-9947-DEADDEADBEEF"),
            (1, 12, "00000000-0101-1212-8E5F-DEADDEADBEEF"),
            (2, 12, "00000000-0202-1212-AA3B-DEADDEADBEEF"),
            (3, 12, "00000000-0303-1212-96C2-DEADDEADBEEF"),
            (4, 12, "00000000-0404-1212-A35C-DEADDEADBEEF"),
            (5, 12, "00000000-0505-1212-8B18-DEADDEADBEEF"),
            (6, 12, "00000000-0606-1212-9FF6-DEADDEADBEEF"),
            (7, 12, "00000000-0707-1212-B8AF-DEADDEADBEEF"),
            (8, 12, "00000000-0808-1212-9578-DEADDEADBEEF"),
            (9, 12, "00000000-0909-1212-BDAB-DEADDEADBEEF"),
            (10, 12, "00000000-1010-1212-AF3A-DEADDEADBEEF"),
            (11, 12, "00000000-1111-1212-BB13-DEADDEADBEEF"),

            (0, 13, "00000000-0000-1313-A322-DEADDEADBEEF"),
            (1, 13, "00000000-0101-1313-BF09-DEADDEADBEEF"),
            (2, 13, "00000000-0202-1313-AC06-DEADDEADBEEF"),
            (3, 13, "00000000-0303-1313-86D3-DEADDEADBEEF"),
            (4, 13, "00000000-0404-1313-967B-DEADDEADBEEF"),
            (5, 13, "00000000-0505-1313-821A-DEADDEADBEEF"),
            (6, 13, "00000000-0606-1313-85E6-DEADDEADBEEF"),
            (7, 13, "00000000-0707-1313-9722-DEADDEADBEEF"),
            (8, 13, "00000000-0808-1313-A82B-DEADDEADBEEF"),
            (9, 13, "00000000-0909-1313-B174-DEADDEADBEEF"),
            (10, 13, "00000000-1010-1313-AC35-DEADDEADBEEF"),
            (11, 13, "00000000-1111-1313-8719-DEADDEADBEEF"),
            (12, 13, "00000000-1212-1313-ACEE-DEADDEADBEEF"),

            (0, 14, "00000000-0000-1414-A81F-DEADDEADBEEF"),
            (1, 14, "00000000-0101-1414-B539-DEADDEADBEEF"),
            (2, 14, "00000000-0202-1414-AB90-DEADDEADBEEF"),
            (3, 14, "00000000-0303-1414-98EA-DEADDEADBEEF"),
            (4, 14, "00000000-0404-1414-A27D-DEADDEADBEEF"),
            (5, 14, "00000000-0505-1414-BC2E-DEADDEADBEEF"),
            (6, 14, "00000000-0606-1414-ABC7-DEADDEADBEEF"),
            (7, 14, "00000000-0707-1414-8D6F-DEADDEADBEEF"),
            (8, 14, "00000000-0808-1414-A254-DEADDEADBEEF"),
            (9, 14, "00000000-0909-1414-B4F0-DEADDEADBEEF"),
            (10, 14, "00000000-1010-1414-84C6-DEADDEADBEEF"),
            (11, 14, "00000000-1111-1414-964B-DEADDEADBEEF"),
            (12, 14, "00000000-1212-1414-8A62-DEADDEADBEEF"),
            (13, 14, "00000000-1313-1414-975D-DEADDEADBEEF"),

            (0, 15, "00000000-0000-1515-AE2C-DEADDEADBEEF"),
            (1, 15, "00000000-0101-1515-A232-DEADDEADBEEF"),
            (2, 15, "00000000-0202-1515-8212-DEADDEADBEEF"),
            (3, 15, "00000000-0303-1515-B1B3-DEADDEADBEEF"),
            (4, 15, "00000000-0404-1515-A791-DEADDEADBEEF"),
            (5, 15, "00000000-0505-1515-92C3-DEADDEADBEEF"),
            (6, 15, "00000000-0606-1515-9A88-DEADDEADBEEF"),
            (7, 15, "00000000-0707-1515-894D-DEADDEADBEEF"),
            (8, 15, "00000000-0808-1515-9A62-DEADDEADBEEF"),
            (9, 15, "00000000-0909-1515-9FD0-DEADDEADBEEF"),
            (10, 15, "00000000-1010-1515-8979-DEADDEADBEEF"),
            (11, 15, "00000000-1111-1515-97E0-DEADDEADBEEF"),
            (12, 15, "00000000-1212-1515-AED2-DEADDEADBEEF"),
            (13, 15, "00000000-1313-1515-882F-DEADDEADBEEF"),
            (14, 15, "00000000-1414-1515-A897-DEADDEADBEEF"),

            (0, 16, "00000000-0000-1616-AA5C-DEADDEADBEEF"),
            (1, 16, "00000000-0101-1616-8430-DEADDEADBEEF"),
            (2, 16, "00000000-0202-1616-A500-DEADDEADBEEF"),
            (3, 16, "00000000-0303-1616-BB01-DEADDEADBEEF"),
            (4, 16, "00000000-0404-1616-B663-DEADDEADBEEF"),
            (5, 16, "00000000-0505-1616-8E56-DEADDEADBEEF"),
            (6, 16, "00000000-0606-1616-8883-DEADDEADBEEF"),
            (7, 16, "00000000-0707-1616-8DDF-DEADDEADBEEF"),
            (8, 16, "00000000-0808-1616-8ADD-DEADDEADBEEF"),
            (9, 16, "00000000-0909-1616-A1E7-DEADDEADBEEF"),
            (10, 16, "00000000-1010-1616-A7A3-DEADDEADBEEF"),
            (11, 16, "00000000-1111-1616-B54B-DEADDEADBEEF"),
            (12, 16, "00000000-1212-1616-A5B8-DEADDEADBEEF"),
            (13, 16, "00000000-1313-1616-A606-DEADDEADBEEF"),
            (14, 16, "00000000-1414-1616-A611-DEADDEADBEEF"),
            (15, 16, "00000000-1515-1616-B304-DEADDEADBEEF"),

            (0, 17, "00000000-0000-1717-A681-DEADDEADBEEF"),
            (1, 17, "00000000-0101-1717-8C8B-DEADDEADBEEF"),
            (2, 17, "00000000-0202-1717-B993-DEADDEADBEEF"),
            (3, 17, "00000000-0303-1717-A2E9-DEADDEADBEEF"),
            (4, 17, "00000000-0404-1717-89F2-DEADDEADBEEF"),
            (5, 17, "00000000-0505-1717-8D08-DEADDEADBEEF"),
            (6, 17, "00000000-0606-1717-9755-DEADDEADBEEF"),
            (7, 17, "00000000-0707-1717-B3E9-DEADDEADBEEF"),
            (8, 17, "00000000-0808-1717-9E20-DEADDEADBEEF"),
            (9, 17, "00000000-0909-1717-8E29-DEADDEADBEEF"),
            (10, 17, "00000000-1010-1717-8EFA-DEADDEADBEEF"),
            (11, 17, "00000000-1111-1717-A249-DEADDEADBEEF"),
            (12, 17, "00000000-1212-1717-9B9E-DEADDEADBEEF"),
            (13, 17, "00000000-1313-1717-98F2-DEADDEADBEEF"),
            (14, 17, "00000000-1414-1717-B6D8-DEADDEADBEEF"),
            (15, 17, "00000000-1515-1717-BD94-DEADDEADBEEF"),
            (16, 17, "00000000-1616-1717-A03D-DEADDEADBEEF"),

            (0, 18, "00000000-0000-1818-8CF4-DEADDEADBEEF"),
            (1, 18, "00000000-0101-1818-BB09-DEADDEADBEEF"),
            (2, 18, "00000000-0202-1818-9424-DEADDEADBEEF"),
            (3, 18, "00000000-0303-1818-AE06-DEADDEADBEEF"),
            (4, 18, "00000000-0404-1818-B718-DEADDEADBEEF"),
            (5, 18, "00000000-0505-1818-945D-DEADDEADBEEF"),
            (6, 18, "00000000-0606-1818-A453-DEADDEADBEEF"),
            (7, 18, "00000000-0707-1818-B192-DEADDEADBEEF"),
            (8, 18, "00000000-0808-1818-8F9F-DEADDEADBEEF"),
            (9, 18, "00000000-0909-1818-87A6-DEADDEADBEEF"),
            (10, 18, "00000000-1010-1818-8AC7-DEADDEADBEEF"),
            (11, 18, "00000000-1111-1818-907E-DEADDEADBEEF"),
            (12, 18, "00000000-1212-1818-A552-DEADDEADBEEF"),
            (13, 18, "00000000-1313-1818-8746-DEADDEADBEEF"),
            (14, 18, "00000000-1414-1818-A327-DEADDEADBEEF"),
            (15, 18, "00000000-1515-1818-83D7-DEADDEADBEEF"),
            (16, 18, "00000000-1616-1818-8066-DEADDEADBEEF"),
            (17, 18, "00000000-1717-1818-A46C-DEADDEADBEEF"),

            (0, 19, "00000000-0000-1919-8252-DEADDEADBEEF"),
            (1, 19, "00000000-0101-1919-8F70-DEADDEADBEEF"),
            (2, 19, "00000000-0202-1919-9334-DEADDEADBEEF"),
            (3, 19, "00000000-0303-1919-BC4B-DEADDEADBEEF"),
            (4, 19, "00000000-0404-1919-90C9-DEADDEADBEEF"),
            (5, 19, "00000000-0505-1919-B476-DEADDEADBEEF"),
            (6, 19, "00000000-0606-1919-9953-DEADDEADBEEF"),
            (7, 19, "00000000-0707-1919-B47F-DEADDEADBEEF"),
            (8, 19, "00000000-0808-1919-B153-DEADDEADBEEF"),
            (9, 19, "00000000-0909-1919-916D-DEADDEADBEEF"),
            (10, 19, "00000000-1010-1919-8EA4-DEADDEADBEEF"),
            (11, 19, "00000000-1111-1919-A837-DEADDEADBEEF"),
            (12, 19, "00000000-1212-1919-BA57-DEADDEADBEEF"),
            (13, 19, "00000000-1313-1919-809C-DEADDEADBEEF"),
            (14, 19, "00000000-1414-1919-93AC-DEADDEADBEEF"),
            (15, 19, "00000000-1515-1919-9875-DEADDEADBEEF"),
            (16, 19, "00000000-1616-1919-8A0F-DEADDEADBEEF"),
            (17, 19, "00000000-1717-1919-BC66-DEADDEADBEEF"),
            (18, 19, "00000000-1818-1919-B1C3-DEADDEADBEEF"),

            (0, 20, "00000000-0000-2020-BCC9-DEADDEADBEEF"),
            (1, 20, "00000000-0101-2020-9296-DEADDEADBEEF"),
            (2, 20, "00000000-0202-2020-AC51-DEADDEADBEEF"),
            (3, 20, "00000000-0303-2020-9F33-DEADDEADBEEF"),
            (4, 20, "00000000-0404-2020-9CDD-DEADDEADBEEF"),
            (5, 20, "00000000-0505-2020-80A0-DEADDEADBEEF"),
            (6, 20, "00000000-0606-2020-A077-DEADDEADBEEF"),
            (7, 20, "00000000-0707-2020-8993-DEADDEADBEEF"),
            (8, 20, "00000000-0808-2020-A58E-DEADDEADBEEF"),
            (9, 20, "00000000-0909-2020-B6C9-DEADDEADBEEF"),
            (10, 20, "00000000-1010-2020-8F71-DEADDEADBEEF"),
            (11, 20, "00000000-1111-2020-86B3-DEADDEADBEEF"),
            (12, 20, "00000000-1212-2020-AF23-DEADDEADBEEF"),
            (13, 20, "00000000-1313-2020-80B0-DEADDEADBEEF"),
            (14, 20, "00000000-1414-2020-962D-DEADDEADBEEF"),
            (15, 20, "00000000-1515-2020-8267-DEADDEADBEEF"),
            (16, 20, "00000000-1616-2020-BD73-DEADDEADBEEF"),
            (17, 20, "00000000-1717-2020-90CB-DEADDEADBEEF"),
            (18, 20, "00000000-1818-2020-87AF-DEADDEADBEEF"),
            (19, 20, "00000000-1919-2020-8E86-DEADDEADBEEF"),

            (0, 21, "00000000-0000-2121-9204-DEADDEADBEEF"),
            (1, 21, "00000000-0101-2121-9378-DEADDEADBEEF"),
            (2, 21, "00000000-0202-2121-A241-DEADDEADBEEF"),
            (3, 21, "00000000-0303-2121-AE2B-DEADDEADBEEF"),
            (4, 21, "00000000-0404-2121-B6CB-DEADDEADBEEF"),
            (5, 21, "00000000-0505-2121-A7A4-DEADDEADBEEF"),
            (6, 21, "00000000-0606-2121-BB58-DEADDEADBEEF"),
            (7, 21, "00000000-0707-2121-B413-DEADDEADBEEF"),
            (8, 21, "00000000-0808-2121-8E00-DEADDEADBEEF"),
            (9, 21, "00000000-0909-2121-900A-DEADDEADBEEF"),
            (10, 21, "00000000-1010-2121-9617-DEADDEADBEEF"),
            (11, 21, "00000000-1111-2121-BFCC-DEADDEADBEEF"),
            (12, 21, "00000000-1212-2121-B104-DEADDEADBEEF"),
            (13, 21, "00000000-1313-2121-9C4C-DEADDEADBEEF"),
            (14, 21, "00000000-1414-2121-97BE-DEADDEADBEEF"),
            (15, 21, "00000000-1515-2121-A10D-DEADDEADBEEF"),
            (16, 21, "00000000-1616-2121-9ACC-DEADDEADBEEF"),
            (17, 21, "00000000-1717-2121-8C64-DEADDEADBEEF"),
            (18, 21, "00000000-1818-2121-91CE-DEADDEADBEEF"),
            (19, 21, "00000000-1919-2121-BC00-DEADDEADBEEF"),
            (20, 21, "00000000-2020-2121-8EAF-DEADDEADBEEF"),

            (0, 22, "00000000-0000-2222-8D69-DEADDEADBEEF"),
            (1, 22, "00000000-0101-2222-8F68-DEADDEADBEEF"),
            (2, 22, "00000000-0202-2222-815A-DEADDEADBEEF"),
            (3, 22, "00000000-0303-2222-9447-DEADDEADBEEF"),
            (4, 22, "00000000-0404-2222-9A14-DEADDEADBEEF"),
            (5, 22, "00000000-0505-2222-8FD5-DEADDEADBEEF"),
            (6, 22, "00000000-0606-2222-9715-DEADDEADBEEF"),
            (7, 22, "00000000-0707-2222-A243-DEADDEADBEEF"),
            (8, 22, "00000000-0808-2222-B50C-DEADDEADBEEF"),
            (9, 22, "00000000-0909-2222-B703-DEADDEADBEEF"),
            (10, 22, "00000000-1010-2222-97A7-DEADDEADBEEF"),
            (11, 22, "00000000-1111-2222-8B4A-DEADDEADBEEF"),
            (12, 22, "00000000-1212-2222-A3E7-DEADDEADBEEF"),
            (13, 22, "00000000-1313-2222-BC1F-DEADDEADBEEF"),
            (14, 22, "00000000-1414-2222-AD73-DEADDEADBEEF"),
            (15, 22, "00000000-1515-2222-974C-DEADDEADBEEF"),
            (16, 22, "00000000-1616-2222-8A99-DEADDEADBEEF"),
            (17, 22, "00000000-1717-2222-A9A2-DEADDEADBEEF"),
            (18, 22, "00000000-1818-2222-B9F7-DEADDEADBEEF"),
            (19, 22, "00000000-1919-2222-A311-DEADDEADBEEF"),
            (20, 22, "00000000-2020-2222-89FA-DEADDEADBEEF"),
            (21, 22, "00000000-2121-2222-8BAF-DEADDEADBEEF"),

            (0, 23, "00000000-0000-2323-921A-DEADDEADBEEF"),
            (1, 23, "00000000-0101-2323-B848-DEADDEADBEEF"),
            (2, 23, "00000000-0202-2323-A1AA-DEADDEADBEEF"),
            (3, 23, "00000000-0303-2323-A42B-DEADDEADBEEF"),
            (4, 23, "00000000-0404-2323-893D-DEADDEADBEEF"),
            (5, 23, "00000000-0505-2323-A2BF-DEADDEADBEEF"),
            (6, 23, "00000000-0606-2323-8200-DEADDEADBEEF"),
            (7, 23, "00000000-0707-2323-8F17-DEADDEADBEEF"),
            (8, 23, "00000000-0808-2323-A650-DEADDEADBEEF"),
            (9, 23, "00000000-0909-2323-840B-DEADDEADBEEF"),
            (10, 23, "00000000-1010-2323-9D7D-DEADDEADBEEF"),
            (11, 23, "00000000-1111-2323-BE8B-DEADDEADBEEF"),
            (12, 23, "00000000-1212-2323-BDBE-DEADDEADBEEF"),
            (13, 23, "00000000-1313-2323-B930-DEADDEADBEEF"),
            (14, 23, "00000000-1414-2323-9317-DEADDEADBEEF"),
            (15, 23, "00000000-1515-2323-8586-DEADDEADBEEF"),
            (16, 23, "00000000-1616-2323-9FE0-DEADDEADBEEF"),
            (17, 23, "00000000-1717-2323-8AE4-DEADDEADBEEF"),
            (18, 23, "00000000-1818-2323-A41C-DEADDEADBEEF"),
            (19, 23, "00000000-1919-2323-A495-DEADDEADBEEF"),
            (20, 23, "00000000-2020-2323-A0D1-DEADDEADBEEF"),
            (21, 23, "00000000-2121-2323-AABF-DEADDEADBEEF"),
            (22, 23, "00000000-2222-2323-9892-DEADDEADBEEF"),

            (0, 24, "00000000-0000-2424-BEC0-DEADDEADBEEF"),
            (1, 24, "00000000-0101-2424-A568-DEADDEADBEEF"),
            (2, 24, "00000000-0202-2424-93DC-DEADDEADBEEF"),
            (3, 24, "00000000-0303-2424-AC6C-DEADDEADBEEF"),
            (4, 24, "00000000-0404-2424-9CF6-DEADDEADBEEF"),
            (5, 24, "00000000-0505-2424-BC3F-DEADDEADBEEF"),
            (6, 24, "00000000-0606-2424-9D82-DEADDEADBEEF"),
            (7, 24, "00000000-0707-2424-B8F3-DEADDEADBEEF"),
            (8, 24, "00000000-0808-2424-9FD0-DEADDEADBEEF"),
            (9, 24, "00000000-0909-2424-82B8-DEADDEADBEEF"),
            (10, 24, "00000000-1010-2424-B71E-DEADDEADBEEF"),
            (11, 24, "00000000-1111-2424-8C00-DEADDEADBEEF"),
            (12, 24, "00000000-1212-2424-8CFC-DEADDEADBEEF"),
            (13, 24, "00000000-1313-2424-809A-DEADDEADBEEF"),
            (14, 24, "00000000-1414-2424-BE71-DEADDEADBEEF"),
            (15, 24, "00000000-1515-2424-8152-DEADDEADBEEF"),
            (16, 24, "00000000-1616-2424-B9AD-DEADDEADBEEF"),
            (17, 24, "00000000-1717-2424-B48D-DEADDEADBEEF"),
            (18, 24, "00000000-1818-2424-9B8B-DEADDEADBEEF"),
            (19, 24, "00000000-1919-2424-97CE-DEADDEADBEEF"),
            (20, 24, "00000000-2020-2424-BC13-DEADDEADBEEF"),
            (21, 24, "00000000-2121-2424-A615-DEADDEADBEEF"),
            (22, 24, "00000000-2222-2424-8395-DEADDEADBEEF"),
            (23, 24, "00000000-2323-2424-80E4-DEADDEADBEEF"),

            (0, 25, "00000000-0000-2525-81ED-DEADDEADBEEF"),
            (1, 25, "00000000-0101-2525-9D48-DEADDEADBEEF"),
            (2, 25, "00000000-0202-2525-850A-DEADDEADBEEF"),
            (3, 25, "00000000-0303-2525-896C-DEADDEADBEEF"),
            (4, 25, "00000000-0404-2525-B29D-DEADDEADBEEF"),
            (5, 25, "00000000-0505-2525-9510-DEADDEADBEEF"),
            (6, 25, "00000000-0606-2525-B2C9-DEADDEADBEEF"),
            (7, 25, "00000000-0707-2525-AC47-DEADDEADBEEF"),
            (8, 25, "00000000-0808-2525-A2C1-DEADDEADBEEF"),
            (9, 25, "00000000-0909-2525-B00E-DEADDEADBEEF"),
            (10, 25, "00000000-1010-2525-8F68-DEADDEADBEEF"),
            (11, 25, "00000000-1111-2525-9AF2-DEADDEADBEEF"),
            (12, 25, "00000000-1212-2525-873E-DEADDEADBEEF"),
            (13, 25, "00000000-1313-2525-8254-DEADDEADBEEF"),
            (14, 25, "00000000-1414-2525-8F57-DEADDEADBEEF"),
            (15, 25, "00000000-1515-2525-97D5-DEADDEADBEEF"),
            (16, 25, "00000000-1616-2525-AFA0-DEADDEADBEEF"),
            (17, 25, "00000000-1717-2525-BCD3-DEADDEADBEEF"),
            (18, 25, "00000000-1818-2525-9D89-DEADDEADBEEF"),
            (19, 25, "00000000-1919-2525-B63F-DEADDEADBEEF"),
            (20, 25, "00000000-2020-2525-9D12-DEADDEADBEEF"),
            (21, 25, "00000000-2121-2525-994E-DEADDEADBEEF"),
            (22, 25, "00000000-2222-2525-AEE7-DEADDEADBEEF"),
            (23, 25, "00000000-2323-2525-B39D-DEADDEADBEEF"),
            (24, 25, "00000000-2424-2525-8DB3-DEADDEADBEEF"),

            (0, 26, "00000000-0000-2626-8BF1-DEADDEADBEEF"),
            (1, 26, "00000000-0101-2626-9ED7-DEADDEADBEEF"),
            (2, 26, "00000000-0202-2626-BA76-DEADDEADBEEF"),
            (3, 26, "00000000-0303-2626-B451-DEADDEADBEEF"),
            (4, 26, "00000000-0404-2626-9E89-DEADDEADBEEF"),
            (5, 26, "00000000-0505-2626-B8E6-DEADDEADBEEF"),
            (6, 26, "00000000-0606-2626-8DE6-DEADDEADBEEF"),
            (7, 26, "00000000-0707-2626-9090-DEADDEADBEEF"),
            (8, 26, "00000000-0808-2626-86B9-DEADDEADBEEF"),
            (9, 26, "00000000-0909-2626-BB26-DEADDEADBEEF"),
            (10, 26, "00000000-1010-2626-8CB3-DEADDEADBEEF"),
            (11, 26, "00000000-1111-2626-B361-DEADDEADBEEF"),
            (12, 26, "00000000-1212-2626-8587-DEADDEADBEEF"),
            (13, 26, "00000000-1313-2626-ABB9-DEADDEADBEEF"),
            (14, 26, "00000000-1414-2626-B203-DEADDEADBEEF"),
            (15, 26, "00000000-1515-2626-B28F-DEADDEADBEEF"),
            (16, 26, "00000000-1616-2626-9B5C-DEADDEADBEEF"),
            (17, 26, "00000000-1717-2626-AA6B-DEADDEADBEEF"),
            (18, 26, "00000000-1818-2626-A98B-DEADDEADBEEF"),
            (19, 26, "00000000-1919-2626-9A98-DEADDEADBEEF"),
            (20, 26, "00000000-2020-2626-B783-DEADDEADBEEF"),
            (21, 26, "00000000-2121-2626-A022-DEADDEADBEEF"),
            (22, 26, "00000000-2222-2626-AFF3-DEADDEADBEEF"),
            (23, 26, "00000000-2323-2626-A446-DEADDEADBEEF"),
            (24, 26, "00000000-2424-2626-A753-DEADDEADBEEF"),
            (25, 26, "00000000-2525-2626-9791-DEADDEADBEEF"),

            (0, 27, "00000000-0000-2727-93F7-DEADDEADBEEF"),
            (1, 27, "00000000-0101-2727-8F4E-DEADDEADBEEF"),
            (2, 27, "00000000-0202-2727-B1EF-DEADDEADBEEF"),
            (3, 27, "00000000-0303-2727-A285-DEADDEADBEEF"),
            (4, 27, "00000000-0404-2727-8AB2-DEADDEADBEEF"),
            (5, 27, "00000000-0505-2727-8FFA-DEADDEADBEEF"),
            (6, 27, "00000000-0606-2727-9643-DEADDEADBEEF"),
            (7, 27, "00000000-0707-2727-A3AD-DEADDEADBEEF"),
            (8, 27, "00000000-0808-2727-ACB6-DEADDEADBEEF"),
            (9, 27, "00000000-0909-2727-B6F6-DEADDEADBEEF"),
            (10, 27, "00000000-1010-2727-9A52-DEADDEADBEEF"),
            (11, 27, "00000000-1111-2727-8245-DEADDEADBEEF"),
            (12, 27, "00000000-1212-2727-B178-DEADDEADBEEF"),
            (13, 27, "00000000-1313-2727-A9B7-DEADDEADBEEF"),
            (14, 27, "00000000-1414-2727-BA3D-DEADDEADBEEF"),
            (15, 27, "00000000-1515-2727-A2CC-DEADDEADBEEF"),
            (16, 27, "00000000-1616-2727-B2DF-DEADDEADBEEF"),
            (17, 27, "00000000-1717-2727-953D-DEADDEADBEEF"),
            (18, 27, "00000000-1818-2727-BDA7-DEADDEADBEEF"),
            (19, 27, "00000000-1919-2727-AA5F-DEADDEADBEEF"),
            (20, 27, "00000000-2020-2727-9988-DEADDEADBEEF"),
            (21, 27, "00000000-2121-2727-9497-DEADDEADBEEF"),
            (22, 27, "00000000-2222-2727-9DE4-DEADDEADBEEF"),
            (23, 27, "00000000-2323-2727-B7B4-DEADDEADBEEF"),
            (24, 27, "00000000-2424-2727-B1C5-DEADDEADBEEF"),
            (25, 27, "00000000-2525-2727-9A90-DEADDEADBEEF"),
            (26, 27, "00000000-2626-2727-A0D7-DEADDEADBEEF"),

            (0, 28, "00000000-0000-2828-A75D-DEADDEADBEEF"),
            (1, 28, "00000000-0101-2828-8064-DEADDEADBEEF"),
            (2, 28, "00000000-0202-2828-A21B-DEADDEADBEEF"),
            (3, 28, "00000000-0303-2828-80F1-DEADDEADBEEF"),
            (4, 28, "00000000-0404-2828-B0DB-DEADDEADBEEF"),
            (5, 28, "00000000-0505-2828-8D4B-DEADDEADBEEF"),
            (6, 28, "00000000-0606-2828-A581-DEADDEADBEEF"),
            (7, 28, "00000000-0707-2828-8F15-DEADDEADBEEF"),
            (8, 28, "00000000-0808-2828-940D-DEADDEADBEEF"),
            (9, 28, "00000000-0909-2828-9F49-DEADDEADBEEF"),
            (10, 28, "00000000-1010-2828-A359-DEADDEADBEEF"),
            (11, 28, "00000000-1111-2828-ACF3-DEADDEADBEEF"),
            (12, 28, "00000000-1212-2828-908F-DEADDEADBEEF"),
            (13, 28, "00000000-1313-2828-BD8B-DEADDEADBEEF"),
            (14, 28, "00000000-1414-2828-ADE1-DEADDEADBEEF"),
            (15, 28, "00000000-1515-2828-99BB-DEADDEADBEEF"),
            (16, 28, "00000000-1616-2828-A46E-DEADDEADBEEF"),
            (17, 28, "00000000-1717-2828-A14B-DEADDEADBEEF"),
            (18, 28, "00000000-1818-2828-8165-DEADDEADBEEF"),
            (19, 28, "00000000-1919-2828-B13B-DEADDEADBEEF"),
            (20, 28, "00000000-2020-2828-98EA-DEADDEADBEEF"),
            (21, 28, "00000000-2121-2828-8C66-DEADDEADBEEF"),
            (22, 28, "00000000-2222-2828-B4DB-DEADDEADBEEF"),
            (23, 28, "00000000-2323-2828-B227-DEADDEADBEEF"),
            (24, 28, "00000000-2424-2828-9B50-DEADDEADBEEF"),
            (25, 28, "00000000-2525-2828-A85B-DEADDEADBEEF"),
            (26, 28, "00000000-2626-2828-93B1-DEADDEADBEEF"),
            (27, 28, "00000000-2727-2828-A0A6-DEADDEADBEEF"),

            (0, 29, "00000000-0000-2929-B43F-DEADDEADBEEF"),
            (1, 29, "00000000-0101-2929-A9E3-DEADDEADBEEF"),
            (2, 29, "00000000-0202-2929-BD43-DEADDEADBEEF"),
            (3, 29, "00000000-0303-2929-AF1D-DEADDEADBEEF"),
            (4, 29, "00000000-0404-2929-94CD-DEADDEADBEEF"),
            (5, 29, "00000000-0505-2929-8AFE-DEADDEADBEEF"),
            (6, 29, "00000000-0606-2929-9445-DEADDEADBEEF"),
            (7, 29, "00000000-0707-2929-AD30-DEADDEADBEEF"),
            (8, 29, "00000000-0808-2929-B995-DEADDEADBEEF"),
            (9, 29, "00000000-0909-2929-BD31-DEADDEADBEEF"),
            (10, 29, "00000000-1010-2929-A8D7-DEADDEADBEEF"),
            (11, 29, "00000000-1111-2929-AE06-DEADDEADBEEF"),
            (12, 29, "00000000-1212-2929-9C93-DEADDEADBEEF"),
            (13, 29, "00000000-1313-2929-B9E7-DEADDEADBEEF"),
            (14, 29, "00000000-1414-2929-ABF0-DEADDEADBEEF"),
            (15, 29, "00000000-1515-2929-B83D-DEADDEADBEEF"),
            (16, 29, "00000000-1616-2929-A25A-DEADDEADBEEF"),
            (17, 29, "00000000-1717-2929-9CCC-DEADDEADBEEF"),
            (18, 29, "00000000-1818-2929-886C-DEADDEADBEEF"),
            (19, 29, "00000000-1919-2929-B785-DEADDEADBEEF"),
            (20, 29, "00000000-2020-2929-8460-DEADDEADBEEF"),
            (21, 29, "00000000-2121-2929-8321-DEADDEADBEEF"),
            (22, 29, "00000000-2222-2929-AC72-DEADDEADBEEF"),
            (23, 29, "00000000-2323-2929-A47B-DEADDEADBEEF"),
            (24, 29, "00000000-2424-2929-92CB-DEADDEADBEEF"),
            (25, 29, "00000000-2525-2929-A0D3-DEADDEADBEEF"),
            (26, 29, "00000000-2626-2929-947E-DEADDEADBEEF"),
            (27, 29, "00000000-2727-2929-BF0A-DEADDEADBEEF"),
            (28, 29, "00000000-2828-2929-BD39-DEADDEADBEEF"),

            (0, 30, "00000000-0000-3030-BFAD-DEADDEADBEEF"),
            (1, 30, "00000000-0101-3030-BC72-DEADDEADBEEF"),
            (2, 30, "00000000-0202-3030-912E-DEADDEADBEEF"),
            (3, 30, "00000000-0303-3030-A220-DEADDEADBEEF"),
            (4, 30, "00000000-0404-3030-A7F1-DEADDEADBEEF"),
            (5, 30, "00000000-0505-3030-A281-DEADDEADBEEF"),
            (6, 30, "00000000-0606-3030-BBF2-DEADDEADBEEF"),
            (7, 30, "00000000-0707-3030-9CDD-DEADDEADBEEF"),
            (8, 30, "00000000-0808-3030-8A68-DEADDEADBEEF"),
            (9, 30, "00000000-0909-3030-9DBB-DEADDEADBEEF"),
            (10, 30, "00000000-1010-3030-906F-DEADDEADBEEF"),
            (11, 30, "00000000-1111-3030-9082-DEADDEADBEEF"),
            (12, 30, "00000000-1212-3030-917D-DEADDEADBEEF"),
            (13, 30, "00000000-1313-3030-93A2-DEADDEADBEEF"),
            (14, 30, "00000000-1414-3030-968B-DEADDEADBEEF"),
            (15, 30, "00000000-1515-3030-BAF5-DEADDEADBEEF"),
            (16, 30, "00000000-1616-3030-B048-DEADDEADBEEF"),
            (17, 30, "00000000-1717-3030-89D8-DEADDEADBEEF"),
            (18, 30, "00000000-1818-3030-B394-DEADDEADBEEF"),
            (19, 30, "00000000-1919-3030-AF6B-DEADDEADBEEF"),
            (20, 30, "00000000-2020-3030-AC9E-DEADDEADBEEF"),
            (21, 30, "00000000-2121-3030-BD96-DEADDEADBEEF"),
            (22, 30, "00000000-2222-3030-A464-DEADDEADBEEF"),
            (23, 30, "00000000-2323-3030-A115-DEADDEADBEEF"),
            (24, 30, "00000000-2424-3030-B735-DEADDEADBEEF"),
            (25, 30, "00000000-2525-3030-A500-DEADDEADBEEF"),
            (26, 30, "00000000-2626-3030-A972-DEADDEADBEEF"),
            (27, 30, "00000000-2727-3030-BE84-DEADDEADBEEF"),
            (28, 30, "00000000-2828-3030-8006-DEADDEADBEEF"),
            (29, 30, "00000000-2929-3030-A617-DEADDEADBEEF"),

            (0, 31, "00000000-0000-3131-A525-DEADDEADBEEF"),
            (1, 31, "00000000-0101-3131-BCDE-DEADDEADBEEF"),
            (2, 31, "00000000-0202-3131-8619-DEADDEADBEEF"),
            (3, 31, "00000000-0303-3131-B99A-DEADDEADBEEF"),
            (4, 31, "00000000-0404-3131-9050-DEADDEADBEEF"),
            (5, 31, "00000000-0505-3131-8BAA-DEADDEADBEEF"),
            (6, 31, "00000000-0606-3131-B242-DEADDEADBEEF"),
            (7, 31, "00000000-0707-3131-82AE-DEADDEADBEEF"),
            (8, 31, "00000000-0808-3131-8C86-DEADDEADBEEF"),
            (9, 31, "00000000-0909-3131-A891-DEADDEADBEEF"),
            (10, 31, "00000000-1010-3131-9A08-DEADDEADBEEF"),
            (11, 31, "00000000-1111-3131-941B-DEADDEADBEEF"),
            (12, 31, "00000000-1212-3131-962F-DEADDEADBEEF"),
            (13, 31, "00000000-1313-3131-8B56-DEADDEADBEEF"),
            (14, 31, "00000000-1414-3131-81A3-DEADDEADBEEF"),
            (15, 31, "00000000-1515-3131-B9F5-DEADDEADBEEF"),
            (16, 31, "00000000-1616-3131-8996-DEADDEADBEEF"),
            (17, 31, "00000000-1717-3131-BE2A-DEADDEADBEEF"),
            (18, 31, "00000000-1818-3131-B4B8-DEADDEADBEEF"),
            (19, 31, "00000000-1919-3131-AA63-DEADDEADBEEF"),
            (20, 31, "00000000-2020-3131-A74D-DEADDEADBEEF"),
            (21, 31, "00000000-2121-3131-B14B-DEADDEADBEEF"),
            (22, 31, "00000000-2222-3131-A2FA-DEADDEADBEEF"),
            (23, 31, "00000000-2323-3131-A51B-DEADDEADBEEF"),
            (24, 31, "00000000-2424-3131-A6BB-DEADDEADBEEF"),
            (25, 31, "00000000-2525-3131-A393-DEADDEADBEEF"),
            (26, 31, "00000000-2626-3131-AF58-DEADDEADBEEF"),
            (27, 31, "00000000-2727-3131-83D8-DEADDEADBEEF"),
            (28, 31, "00000000-2828-3131-90F5-DEADDEADBEEF"),
            (29, 31, "00000000-2929-3131-B89F-DEADDEADBEEF"),
            (30, 31, "00000000-3030-3131-A707-DEADDEADBEEF"),

            (0, 32, "00000000-0000-3232-B086-DEADDEADBEEF"),
            (1, 32, "00000000-0101-3232-BFAE-DEADDEADBEEF"),
            (2, 32, "00000000-0202-3232-8D06-DEADDEADBEEF"),
            (3, 32, "00000000-0303-3232-A424-DEADDEADBEEF"),
            (4, 32, "00000000-0404-3232-A296-DEADDEADBEEF"),
            (5, 32, "00000000-0505-3232-BE62-DEADDEADBEEF"),
            (6, 32, "00000000-0606-3232-A3AB-DEADDEADBEEF"),
            (7, 32, "00000000-0707-3232-8BCA-DEADDEADBEEF"),
            (8, 32, "00000000-0808-3232-9228-DEADDEADBEEF"),
            (9, 32, "00000000-0909-3232-A703-DEADDEADBEEF"),
            (10, 32, "00000000-1010-3232-9E83-DEADDEADBEEF"),
            (11, 32, "00000000-1111-3232-B904-DEADDEADBEEF"),
            (12, 32, "00000000-1212-3232-8DAE-DEADDEADBEEF"),
            (13, 32, "00000000-1313-3232-A2B4-DEADDEADBEEF"),
            (14, 32, "00000000-1414-3232-9725-DEADDEADBEEF"),
            (15, 32, "00000000-1515-3232-8029-DEADDEADBEEF"),
            (16, 32, "00000000-1616-3232-BDF4-DEADDEADBEEF"),
            (17, 32, "00000000-1717-3232-9073-DEADDEADBEEF"),
            (18, 32, "00000000-1818-3232-AC8C-DEADDEADBEEF"),
            (19, 32, "00000000-1919-3232-B968-DEADDEADBEEF"),
            (20, 32, "00000000-2020-3232-B406-DEADDEADBEEF"),
            (21, 32, "00000000-2121-3232-ABEA-DEADDEADBEEF"),
            (22, 32, "00000000-2222-3232-8F73-DEADDEADBEEF"),
            (23, 32, "00000000-2323-3232-884B-DEADDEADBEEF"),
            (24, 32, "00000000-2424-3232-A0A5-DEADDEADBEEF"),
            (25, 32, "00000000-2525-3232-B5FB-DEADDEADBEEF"),
            (26, 32, "00000000-2626-3232-8640-DEADDEADBEEF"),
            (27, 32, "00000000-2727-3232-8334-DEADDEADBEEF"),
            (28, 32, "00000000-2828-3232-A80A-DEADDEADBEEF"),
            (29, 32, "00000000-2929-3232-898B-DEADDEADBEEF"),
            (30, 32, "00000000-3030-3232-B3BE-DEADDEADBEEF"),
            (31, 32, "00000000-3131-3232-B974-DEADDEADBEEF")
        };

        foreach (var l in list)
        {
            if (l.Item1 != l.Item3.DeterminePartitionId(entityPartitionCount: (short)l.Item2))
            {
                throw new Exception();
            }
        }
        Console.WriteLine("OK");
    }

    public static void Main()
    {
        BruteForce();
        TestCompliance();
        foreach (var x in Enumerable.Range(0, 32).Select(desiredPartitionId => BruteForceGetFor(desiredPartitionId: (short)desiredPartitionId, partitionCount: 32)))
        {
            Console.WriteLine(x);
        }
    }
}