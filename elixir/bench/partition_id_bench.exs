# Benchmark for AzurePartitionId

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

partition_counts = [1, 8, 16, 32]

# Generate test cases
test_cases = 
  for key <- test_keys,
      count <- partition_counts do
    {key, count}
  end

Benchee.run(
  %{
    "determine_partition_id" => fn ->
      Enum.each(test_cases, fn {key, count} ->
        AzurePartitionId.determine_partition_id(key, count)
      end)
    end
  },
  time: 5,
  memory_time: 2,
  warmup: 2,
  formatters: [
    Benchee.Formatters.Console
  ]
)

# Individual operation benchmarks
IO.puts("\n=== Individual Operation Benchmarks ===\n")

Benchee.run(
  %{
    "single call (8 partitions)" => fn ->
      AzurePartitionId.determine_partition_id("00000000-0303-0808-B4C9-DEADDEADBEEF", 8)
    end,
    "single call (32 partitions)" => fn ->
      AzurePartitionId.determine_partition_id("00000000-3131-3232-B974-DEADDEADBEEF", 32)
    end,
    "short key (8 partitions)" => fn ->
      AzurePartitionId.determine_partition_id("short", 8)
    end,
    "long key (8 partitions)" => fn ->
      AzurePartitionId.determine_partition_id(String.duplicate("long-key-", 100), 8)
    end
  },
  time: 5,
  memory_time: 2,
  warmup: 2
)
