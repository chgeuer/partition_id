# Benchmark comparison between original and optimized versions

# Test data
test_keys = [
  "00000000-0000-0101-9A83-DEADDEADBEEF",
  "00000000-0303-0808-B4C9-DEADDEADBEEF",
  "00000000-1515-3232-8029-DEADDEADBEEF",
  "00000000-3131-3232-B974-DEADDEADBEEF",
  "SOME-RANDOM-KEY-12345",
  "another-test-key-with-more-length",
  "short",
  String.duplicate("long-key-", 100)
]

IO.puts("=== Verifying Correctness First ===\n")

# Verify both implementations produce the same results
all_match = 
  Enum.all?(test_keys, fn key ->
    Enum.all?([1, 8, 16, 32], fn count ->
      original = AzurePartitionId.determine_partition_id(key, count)
      optimized = AzurePartitionIdOptimized.determine_partition_id(key, count)
      
      if original != optimized do
        IO.puts("MISMATCH: key=#{key}, count=#{count}, original=#{original}, optimized=#{optimized}")
        false
      else
        true
      end
    end)
  end)

if all_match do
  IO.puts("✅ All test cases match between original and optimized versions\n")
else
  IO.puts("❌ Some test cases don't match! Stopping benchmark.\n")
  System.halt(1)
end

IO.puts("=== Short Keys (UUID-like) ===\n")

Benchee.run(
  %{
    "original (short, 8 parts)" => fn ->
      AzurePartitionId.determine_partition_id("00000000-0303-0808-B4C9-DEADDEADBEEF", 8)
    end,
    "optimized (short, 8 parts)" => fn ->
      AzurePartitionIdOptimized.determine_partition_id("00000000-0303-0808-B4C9-DEADDEADBEEF", 8)
    end,
    "original (short, 32 parts)" => fn ->
      AzurePartitionId.determine_partition_id("00000000-3131-3232-B974-DEADDEADBEEF", 32)
    end,
    "optimized (short, 32 parts)" => fn ->
      AzurePartitionIdOptimized.determine_partition_id("00000000-3131-3232-B974-DEADDEADBEEF", 32)
    end
  },
  time: 3,
  memory_time: 1,
  warmup: 1
)

IO.puts("\n=== Very Short Keys ===\n")

Benchee.run(
  %{
    "original (very short)" => fn ->
      AzurePartitionId.determine_partition_id("short", 8)
    end,
    "optimized (very short)" => fn ->
      AzurePartitionIdOptimized.determine_partition_id("short", 8)
    end
  },
  time: 3,
  memory_time: 1,
  warmup: 1
)

IO.puts("\n=== Long Keys ===\n")

long_key = String.duplicate("long-key-", 100)

Benchee.run(
  %{
    "original (long)" => fn ->
      AzurePartitionId.determine_partition_id(long_key, 8)
    end,
    "optimized (long)" => fn ->
      AzurePartitionIdOptimized.determine_partition_id(long_key, 8)
    end
  },
  time: 3,
  memory_time: 1,
  warmup: 1
)

IO.puts("\n=== Batch Processing (32 operations) ===\n")

test_cases = 
  for key <- test_keys,
      count <- [8, 32] do
    {key, count}
  end

Benchee.run(
  %{
    "original (batch)" => fn ->
      Enum.each(test_cases, fn {key, count} ->
        AzurePartitionId.determine_partition_id(key, count)
      end)
    end,
    "optimized (batch)" => fn ->
      Enum.each(test_cases, fn {key, count} ->
        AzurePartitionIdOptimized.determine_partition_id(key, count)
      end)
    end
  },
  time: 3,
  memory_time: 1,
  warmup: 1
)
