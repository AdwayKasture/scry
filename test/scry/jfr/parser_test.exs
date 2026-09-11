defmodule Scry.Jfr.ParserTest do
  use ExUnit.Case

  alias Scry.Jfr.Parser

  defp fixture_path, do: Path.join(__DIR__, "../../fixtures/sample.jfr") |> Path.expand()

  test "parse_summary/1 returns chunk and event counts from a JFR file" do
    summary = Parser.parse_summary(fixture_path())

    assert summary.chunk_count >= 1
    assert is_map(summary.event_counts)
    assert map_size(summary.event_counts) > 0
  end

  test "list_event_types/1 returns event class names present in the recording" do
    types = Parser.list_event_types(fixture_path())

    assert "jdk.ExecutionSample" in types
    assert "jdk.ThreadCPULoad" in types
  end

  test "extract_events/2 returns granular events with stack traces" do
    [sample | _] = Parser.extract_events(fixture_path(), ["jdk.ExecutionSample"])

    assert sample["event"] == "jdk.ExecutionSample"
    assert is_integer(sample["timestamp"])
    assert sample["thread"] == "main"

    [top_frame | _] = sample["stack_trace"]
    assert top_frame["class"] == "CpuWork"
    assert top_frame["method"] == "compute"
    assert is_integer(top_frame["line"])

    assert is_map(sample["fields"])
  end
end
