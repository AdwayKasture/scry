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

  @doc """
  Returns the sorted list of unique event type names found in the JFR file.
  """
  def list_event_types(path) do
    list_event_types_nif(path)
  end

  @doc """
  Extracts granular events from the JFR file.

  `event_names` is a list of event class names to include. Pass an empty list
  to return every event.

  Each returned event is a map with:

    * `:event` - event class name (e.g. `"jdk.ExecutionSample"`)
    * `:timestamp` - start time as epoch nanoseconds (when available)
    * `:duration` - duration in nanoseconds (when available)
    * `:thread` - thread name (when available)
    * `:stack_trace` - simplified stack trace as a list of maps with
      `:class`, `:method`, and `:line` (when available)
    * `:fields` - map of the event's remaining fields

  """
  def extract_events(path, event_names \\ []) do
    extract_events_nif(path, event_names)
  end

  defp parse_summary_nif(_path), do: :erlang.nif_error(:nif_not_loaded)
  defp list_event_types_nif(_path), do: :erlang.nif_error(:nif_not_loaded)
  defp extract_events_nif(_path, _event_names), do: :erlang.nif_error(:nif_not_loaded)
end
