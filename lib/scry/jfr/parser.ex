defmodule Scry.Jfr.Parser do
  @moduledoc """
  Rustler NIF for parsing JFR (Java Flight Recorder) chunks.

  The actual parsing is done in the `native/scry_jfr_parser` Rust crate,
  which uses the `jfrs` library to read native JFR files.
  """

  use Rustler,
    otp_app: :scry,
    crate: :scry_jfr_parser

  @doc """
  Returns a summary of the JFR file at `path`.

  The result is a map with:

    * `:chunk_count` - the number of JFR chunks found in the file
    * `:event_counts` - a map of event class name to occurrence count

  """
  def parse_summary(path) do
    {chunk_count, event_counts} = parse_summary_nif(path)
    %{chunk_count: chunk_count, event_counts: event_counts}
  end

  defp parse_summary_nif(_path), do: :erlang.nif_error(:nif_not_loaded)
end
