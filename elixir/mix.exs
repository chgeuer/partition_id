defmodule AzurePartitionId.MixProject do
  use Mix.Project

  def project do
    [
      app: :azure_partition_id,
      version: "0.1.0",
      elixir: "~> 1.19",
      start_permanent: Mix.env() == :prod,
      deps: deps()
    ]
  end

  # Run "mix help compile.app" to learn about applications.
  def application do
    [
      extra_applications: [:logger]
    ]
  end

  # Run "mix help deps" to learn about dependencies.
  defp deps do
    [
      {:credo, "~> 1.7", only: [:dev, :test], runtime: false},
      {:ex_doc, "~> 0.39.1", only: [:dev], runtime: false},
      {:nimble_options, "~> 1.1"}
    ]
  end
end
