defmodule ScryWeb.PageController do
  use ScryWeb, :controller

  def home(conn, _params) do
    render(conn, :home)
  end
end
