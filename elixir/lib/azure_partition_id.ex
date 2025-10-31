defmodule AzurePartitionId do
  @moduledoc """
  Computes partition IDs based on partition keys using a hash-based algorithm.

  This module provides functionality to determine which partition a given partition key
  should be assigned to, based on the total number of partitions available.

  ## Performance

  This implementation is optimized for performance using:
  - Direct binary processing (no charlist conversion)
  - Tuple-based ranges for faster binary search
  - Minimal allocations in hot paths

  Typical performance: 0.4-1.7 μs per operation for standard partition keys.
  """

  import Bitwise

  @max_partition 32_767
  @hash_seed 0xDEADBEEF

  @doc """
  Determines the partition ID for a given partition key and total partition count.

  ## Parameters

    * `partition_key` - The partition key as a string (will be uppercased)
    * `partition_count` - The total number of partitions available (integer)

  ## Examples

      iex> AzurePartitionId.determine_partition_id("00000000-0000-0101-9A83-DEADDEADBEEF", 1)
      0

      iex> AzurePartitionId.determine_partition_id("00000000-0101-0202-B117-DEADDEADBEEF", 2)
      1

  ## Options (when second argument is a keyword list)

    * `:partition_count` - (required) The total number of partitions

  """
  @spec determine_partition_id(String.t(), integer() | keyword()) :: non_neg_integer()
  def determine_partition_id(partition_key, partition_count_or_opts)

  def determine_partition_id(partition_key, partition_count)
      when is_binary(partition_key) and is_integer(partition_count) do
    ranges = get_ranges_tuple(partition_count)
    logical = to_logical(partition_key)
    to_partition_id(ranges, logical)
  end

  def determine_partition_id(partition_key, opts)
      when is_binary(partition_key) and is_list(opts) do
    validated_opts =
      NimbleOptions.validate!(opts, partition_count: [type: :pos_integer, required: true])

    partition_count = validated_opts[:partition_count]
    determine_partition_id(partition_key, partition_count)
  end

  # Get ranges as tuple for faster binary search
  defp get_ranges_tuple(range_count) do
    partitions_per_range_base = div(@max_partition, range_count)
    remaining_partitions = @max_partition - range_count * partitions_per_range_base

    ranges =
      Range.new(0, range_count - 2, 1)
      |> Enum.reduce({[], -1}, fn i, {acc, end_val} ->
        partitions_per_range =
          if i < remaining_partitions do
            partitions_per_range_base + 1
          else
            partitions_per_range_base
          end

        new_end = min(end_val + partitions_per_range, @max_partition - 1)
        {[new_end | acc], new_end}
      end)
      |> elem(0)
      |> Enum.reverse()

    List.to_tuple(ranges ++ [@max_partition - 1])
  end

  # Converts partition key to logical partition number using binary processing
  @spec to_logical(String.t()) :: integer()
  defp to_logical(partition_key) when partition_key == "" or partition_key == nil do
    0
  end

  defp to_logical(partition_key) do
    binary = String.upcase(partition_key)
    {hash1, hash2, _} = hash_binary(binary)
    rem(bxor(hash1, hash2), @max_partition)
  end

  # Hash function working directly on binaries (optimized)
  defp hash_binary(binary) do
    initial = @hash_seed + byte_size(binary)
    a = initial
    b = initial
    c = initial

    {a, b, c} = process_binary_chunks(binary, a, b, c)
    {_a, b, c} = final_mix(a, b, c)

    {c, b, c + (b <<< 32)}
  end

  # Process binary chunks directly (optimized for performance)
  defp process_binary_chunks(binary, a, b, c) when byte_size(binary) > 12 do
    <<a_add::little-unsigned-32, b_add::little-unsigned-32, c_add::little-unsigned-32,
      rest::binary>> = binary

    a = mask32(a + a_add)
    b = mask32(b + b_add)
    c = mask32(c + c_add)

    {a, b, c} = mix(a, b, c)

    process_binary_chunks(rest, a, b, c)
  end

  defp process_binary_chunks(binary, a, b, c) when byte_size(binary) > 0 do
    process_binary_remainder(binary, a, b, c)
  end

  defp process_binary_chunks(_binary, a, b, c) do
    {a, b, c}
  end

  # Process remaining bytes using binary pattern matching
  defp process_binary_remainder(<<b0, b1, b2, b3, b4, b5, b6, b7, b8, b9, b10, b11>>, a, b, c) do
    a = mask32(a + b0 + (b1 <<< 8) + (b2 <<< 16) + (b3 <<< 24))
    b = mask32(b + b4 + (b5 <<< 8) + (b6 <<< 16) + (b7 <<< 24))
    c = mask32(c + b8 + (b9 <<< 8) + (b10 <<< 16) + (b11 <<< 24))
    {a, b, c}
  end

  defp process_binary_remainder(<<b0, b1, b2, b3, b4, b5, b6, b7, b8, b9, b10>>, a, b, c) do
    a = mask32(a + b0 + (b1 <<< 8) + (b2 <<< 16) + (b3 <<< 24))
    b = mask32(b + b4 + (b5 <<< 8) + (b6 <<< 16) + (b7 <<< 24))
    c = mask32(c + b8 + (b9 <<< 8) + (b10 <<< 16))
    {a, b, c}
  end

  defp process_binary_remainder(<<b0, b1, b2, b3, b4, b5, b6, b7, b8, b9>>, a, b, c) do
    a = mask32(a + b0 + (b1 <<< 8) + (b2 <<< 16) + (b3 <<< 24))
    b = mask32(b + b4 + (b5 <<< 8) + (b6 <<< 16) + (b7 <<< 24))
    c = mask32(c + b8 + (b9 <<< 8))
    {a, b, c}
  end

  defp process_binary_remainder(<<b0, b1, b2, b3, b4, b5, b6, b7, b8>>, a, b, c) do
    a = mask32(a + b0 + (b1 <<< 8) + (b2 <<< 16) + (b3 <<< 24))
    b = mask32(b + b4 + (b5 <<< 8) + (b6 <<< 16) + (b7 <<< 24))
    c = mask32(c + b8)
    {a, b, c}
  end

  defp process_binary_remainder(<<b0, b1, b2, b3, b4, b5, b6, b7>>, a, b, c) do
    a = mask32(a + b0 + (b1 <<< 8) + (b2 <<< 16) + (b3 <<< 24))
    b = mask32(b + b4 + (b5 <<< 8) + (b6 <<< 16) + (b7 <<< 24))
    {a, b, c}
  end

  defp process_binary_remainder(<<b0, b1, b2, b3, b4, b5, b6>>, a, b, c) do
    a = mask32(a + b0 + (b1 <<< 8) + (b2 <<< 16) + (b3 <<< 24))
    b = mask32(b + b4 + (b5 <<< 8) + (b6 <<< 16))
    {a, b, c}
  end

  defp process_binary_remainder(<<b0, b1, b2, b3, b4, b5>>, a, b, c) do
    a = mask32(a + b0 + (b1 <<< 8) + (b2 <<< 16) + (b3 <<< 24))
    b = mask32(b + b4 + (b5 <<< 8))
    {a, b, c}
  end

  defp process_binary_remainder(<<b0, b1, b2, b3, b4>>, a, b, c) do
    a = mask32(a + b0 + (b1 <<< 8) + (b2 <<< 16) + (b3 <<< 24))
    b = mask32(b + b4)
    {a, b, c}
  end

  defp process_binary_remainder(<<b0, b1, b2, b3>>, a, b, c) do
    a = mask32(a + b0 + (b1 <<< 8) + (b2 <<< 16) + (b3 <<< 24))
    {a, b, c}
  end

  defp process_binary_remainder(<<b0, b1, b2>>, a, b, c) do
    a = mask32(a + b0 + (b1 <<< 8) + (b2 <<< 16))
    {a, b, c}
  end

  defp process_binary_remainder(<<b0, b1>>, a, b, c) do
    a = mask32(a + b0 + (b1 <<< 8))
    {a, b, c}
  end

  defp process_binary_remainder(<<b0>>, a, b, c) do
    a = mask32(a + b0)
    {a, b, c}
  end

  defp process_binary_remainder(<<>>, a, b, c) do
    {a, b, c}
  end

  # Jenkins hash mix function
  @spec mix(non_neg_integer(), non_neg_integer(), non_neg_integer()) ::
          {non_neg_integer(), non_neg_integer(), non_neg_integer()}
  defp mix(a, b, c) do
    a = mask32(a - c)
    a = mask32(bxor(a, rot(c, 4)))
    c = mask32(c + b)

    b = mask32(b - a)
    b = mask32(bxor(b, rot(a, 6)))
    a = mask32(a + c)

    c = mask32(c - b)
    c = mask32(bxor(c, rot(b, 8)))
    b = mask32(b + a)

    a = mask32(a - c)
    a = mask32(bxor(a, rot(c, 16)))
    c = mask32(c + b)

    b = mask32(b - a)
    b = mask32(bxor(b, rot(a, 19)))
    a = mask32(a + c)

    c = mask32(c - b)
    c = mask32(bxor(c, rot(b, 4)))
    b = mask32(b + a)

    {a, b, c}
  end

  # Jenkins hash final mix
  @spec final_mix(non_neg_integer(), non_neg_integer(), non_neg_integer()) ::
          {non_neg_integer(), non_neg_integer(), non_neg_integer()}
  defp final_mix(a, b, c) do
    c = mask32(bxor(c, b))
    c = mask32(c - rot(b, 14))

    a = mask32(bxor(a, c))
    a = mask32(a - rot(c, 11))

    b = mask32(bxor(b, a))
    b = mask32(b - rot(a, 25))

    c = mask32(bxor(c, b))
    c = mask32(c - rot(b, 16))

    a = mask32(bxor(a, c))
    a = mask32(a - rot(c, 4))

    b = mask32(bxor(b, a))
    b = mask32(b - rot(a, 14))

    c = mask32(bxor(c, b))
    c = mask32(c - rot(b, 24))

    {a, b, c}
  end

  # Rotate left
  @spec rot(non_neg_integer(), non_neg_integer()) :: non_neg_integer()
  defp rot(x, k) do
    mask32(x <<< k ||| x >>> (32 - k))
  end

  # Mask to 32 bits
  @spec mask32(integer()) :: non_neg_integer()
  defp mask32(x) do
    x &&& 0xFFFFFFFF
  end

  # Binary search using tuple for faster access
  @spec to_partition_id(tuple(), integer()) :: non_neg_integer()
  defp to_partition_id(ranges, partition) when is_tuple(ranges) do
    do_tuple_binary_search(ranges, partition, 0, tuple_size(ranges) - 1)
  end

  defp do_tuple_binary_search(_ranges, _partition, lower, upper) when lower >= upper do
    lower
  end

  defp do_tuple_binary_search(ranges, partition, lower, upper) do
    middle = (lower + upper) >>> 1

    if partition > elem(ranges, middle) do
      do_tuple_binary_search(ranges, partition, middle + 1, upper)
    else
      do_tuple_binary_search(ranges, partition, lower, middle)
    end
  end
end
