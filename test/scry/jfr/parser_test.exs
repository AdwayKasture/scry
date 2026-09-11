defmodule Scry.Jfr.ParserTest do
  use ExUnit.Case

  alias Scry.Jfr.Parser

  test "parse_summary/1 returns chunk and event counts from a JFR file" do
    fixture = Path.join(__DIR__, "../../fixtures/sample.jfr") |> Path.expand()

    summary = Parser.parse_summary(fixture)

    assert summary.chunk_count >= 1
    assert is_map(summary.event_counts)
    assert map_size(summary.event_counts) > 0
  end
end
