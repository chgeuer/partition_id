# Elixir Implementation of Partition ID Algorithm

This is an idiomatic Elixir implementation of the partition ID computation algorithm, compatible with the Rust and C# implementations.

## Features

- **Jenkins Hash Function**: Implements the same hash algorithm as C# and Rust versions
- **Binary Search**: Efficiently maps logical partitions to physical partitions
- **NimbleOptions Validation**: Validates options when using keyword list syntax
- **Comprehensive Tests**: 527 test vectors ensuring compatibility
- **Zero Credo Issues**: Passes strict Credo analysis

## Usage

```elixir
# Using integer partition count
AzurePartitionId.determine_partition_id("00000000-0000-0101-9A83-DEADDEADBEEF", 1)
# => 0

# Using keyword options (with NimbleOptions validation)
AzurePartitionId.determine_partition_id(
  "00000000-0101-0202-B117-DEADDEADBEEF",
  partition_count: 2
)
# => 1
```

## Testing

Run all tests:
```bash
mix test
```

Run with formatting and linting:
```bash
just format
```

## Implementation Details

The implementation follows Elixir conventions:

- Uses pattern matching and guard clauses instead of case statements
- Breaks complex functions into smaller, focused functions
- Proper typespecs for all public and private functions
- Documentation with examples
- Validates options using NimbleOptions

## Compatibility

This implementation produces identical results to the C# and Rust versions for all 527 test vectors covering partition counts from 1 to 32.
