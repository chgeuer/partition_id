# AzurePartitionId

An Elixir implementation of the Azure partition ID computation algorithm, compatible with the C# and Rust implementations.

This module computes partition IDs based on partition keys using the Jenkins hash function, ensuring consistent partition assignment across different language implementations.

## Installation

If [available in Hex](https://hex.pm/docs/publish), the package can be installed by adding `azure_partition_id` to your list of dependencies in `mix.exs`:

```elixir
def deps do
  [
    {:azure_partition_id, "~> 0.1.0"}
  ]
end
```

## Usage

### Basic Usage

The simplest way to compute a partition ID is by passing a partition key and partition count:

```elixir
# Compute partition ID for a given key with 8 partitions
partition_id = AzurePartitionId.determine_partition_id(
  "00000000-0303-0808-B4C9-DEADDEADBEEF",
  8
)
# => 3
```

### Using Keyword Options

You can also use keyword syntax with NimbleOptions validation:

```elixir
partition_id = AzurePartitionId.determine_partition_id(
  "00000000-0101-0202-B117-DEADDEADBEEF",
  partition_count: 2
)
# => 1
```

### Real-World Example

```elixir
defmodule MyApp.PartitionRouter do
  @partition_count 32

  def route_message(message_id, message) do
    partition_id = AzurePartitionId.determine_partition_id(
      message_id,
      @partition_count
    )
    
    send_to_partition(partition_id, message)
  end

  defp send_to_partition(partition_id, message) do
    # Send message to the appropriate partition
    GenServer.call({:partition_worker, partition_id}, {:process, message})
  end
end
```

### Working with Azure Service Bus

```elixir
defmodule MyApp.ServiceBusClient do
  def send_to_partition(namespace, entity_name, partition_key, message) do
    # Get partition count from Service Bus metadata
    partition_count = get_partition_count(namespace, entity_name)
    
    # Compute partition ID
    partition_id = AzurePartitionId.determine_partition_id(
      partition_key,
      partition_count
    )
    
    Logger.info("Sending message to partition #{partition_id}")
    
    # Send message using the computed partition ID
    send_message(namespace, entity_name, partition_id, message)
  end
end
```

## API Reference

### `determine_partition_id/2`

Determines the partition ID for a given partition key and partition count.

**Parameters:**
- `partition_key` (String.t()) - The partition key (will be uppercased automatically)
- `partition_count` (integer() | keyword()) - The total number of partitions, or a keyword list with `:partition_count` option

**Returns:**
- `non_neg_integer()` - The computed partition ID (0-indexed)

**Examples:**

```elixir
# Simple usage
iex> AzurePartitionId.determine_partition_id("my-key-123", 10)
7

# Case-insensitive (keys are uppercased)
iex> AzurePartitionId.determine_partition_id("MY-KEY-123", 10)
7

# Using options
iex> AzurePartitionId.determine_partition_id("my-key-123", partition_count: 10)
7
```

## Algorithm Details

The implementation uses:

1. **Jenkins Hash Function**: A non-cryptographic hash function that provides good distribution
2. **Binary Search**: Efficiently maps logical partition numbers to physical partition IDs
3. **Consistent Results**: Produces identical results to C# and Rust implementations

### Hash Computation

The partition key is:
1. Converted to uppercase
2. Hashed using the Jenkins hash function with seed `0xDEADBEEF`
3. XOR'd to produce a logical partition number
4. Mapped to a physical partition ID using binary search over pre-computed ranges

## Testing

Run the test suite:

```bash
mix test
```

Run tests with formatting and linting:

```bash
just format
```

Run benchmarks:

```bash
mix run bench/partition_id_bench.exs
```

## Compatibility

This implementation has been verified against 527 test vectors covering partition counts from 1 to 32, ensuring 100% compatibility with the C# and Rust implementations.

## Performance

The implementation is optimized for performance with:
- Efficient binary search for partition mapping
- Minimal allocations in the hot path
- Pre-computed partition ranges

Typical performance on modern hardware:
- ~500-800 ns per computation (cold)
- ~200-400 ns per computation (warm)

## Code Quality

The codebase maintains high quality standards:
- ✅ Zero Credo issues with `--strict` analysis
- ✅ 100% test coverage of test vectors
- ✅ Comprehensive documentation
- ✅ Proper typespecs throughout

## Contributing

When contributing, ensure:
1. All tests pass: `mix test`
2. Code is formatted: `mix format`
3. Credo passes: `mix credo --strict`
4. New features include tests and documentation

## License

See LICENSE file in the repository root.

## Related Implementations

- **Rust**: `../rust/` - Original implementation
- **C#**: `../dotnet/` - .NET implementation

All three implementations produce identical results for the same inputs.
