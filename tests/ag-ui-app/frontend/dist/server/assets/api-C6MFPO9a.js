import { c as createServerRpc, a as createServerFn } from "../server.js";
import "@tanstack/history";
import "@tanstack/router-core/ssr/client";
import "@tanstack/router-core";
import "node:async_hooks";
import "@tanstack/router-core/ssr/server";
import "h3-v2";
import "tiny-invariant";
import "seroval";
import "react/jsx-runtime";
import "@tanstack/react-router/ssr/server";
import "@tanstack/react-router";
const getApiBase_createServerFn_handler = createServerRpc("cc35041ac8534be1df357d50dcecc9c79d69232d0d2da0a89a0d930737c7ec36", (opts, signal) => getApiBase.__executeServer(opts, signal));
const getApiBase = createServerFn({
  method: "GET"
}).handler(getApiBase_createServerFn_handler, () => {
  const base = process.env.API_BASE || "http://localhost:3001";
  return base.endsWith("/api") ? base : `${base}/api`;
});
export {
  getApiBase_createServerFn_handler
};
