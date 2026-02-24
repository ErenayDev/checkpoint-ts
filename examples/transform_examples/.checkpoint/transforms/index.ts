import { __checkpoint__ } from "../runtime/checkpoint-runtime";
class ApiClient {
  baseUrl;
  defaultHeaders;
  defaultTimeout;
  constructor(baseUrl, defaultTimeout = 1e4) {
    this.baseUrl = __checkpoint__.execute(
      "baseUrl.replace",
      baseUrl.replace,
      [/\/$/, ""],
      baseUrl,
    );
    this.defaultHeaders = { "Content-Type": "application/json" };
    this.defaultTimeout = defaultTimeout;
  }
  setHeader(key, value) {
    this.defaultHeaders[key] = value;
  }
  removeHeader(key) {
    delete this.defaultHeaders[key];
  }
  buildUrl(endpoint, params) {
    const url = new URL(`${this.baseUrl}${endpoint}`);
    if (params) {
      __checkpoint__
        .execute("Object.entries", Object.entries, [params], Object)
        .forEach(([key, value]) => {
          __checkpoint__.execute(
            "url.searchParams.append",
            url.searchParams.append,
            [key, value],
            url.searchParams,
          );
        });
    }
    return __checkpoint__.execute("url.toString", url.toString, [], url);
  }
  async request(method, endpoint, config = {}) {
    const controller = new AbortController();
    const timeoutId = __checkpoint__.execute("setTimeout", setTimeout, [
      () =>
        __checkpoint__.execute(
          "controller.abort",
          controller.abort,
          [],
          controller,
        ),
      config.timeout ?? this.defaultTimeout,
    ]);
    const response = await __checkpoint__.execute("fetch", fetch, [
      __checkpoint__.execute(
        "this.buildUrl",
        this.buildUrl,
        [endpoint, config.params],
        this,
      ),
      {
        method,
        headers: { ...this.defaultHeaders, ...config.headers },
        body: config.body
          ? __checkpoint__.execute(
              "JSON.stringify",
              JSON.stringify,
              [config.body],
              JSON,
            )
          : undefined,
        signal: controller.signal,
      },
    ]);
    __checkpoint__.execute("clearTimeout", clearTimeout, [timeoutId]);
    if (!response.ok) {
      throw new Error(`HTTP ${response.status}: ${response.statusText}`);
    }
    const data = await __checkpoint__.execute(
      "response.json",
      response.json,
      [],
      response,
    );
    return { data, status: response.status, headers: response.headers };
  }
  get(endpoint, config) {
    return __checkpoint__.execute(
      "this.request",
      this.request,
      ["GET", endpoint, config],
      this,
    );
  }
  post(endpoint, body, config) {
    return __checkpoint__.execute(
      "this.request",
      this.request,
      ["POST", endpoint, { ...config, body }],
      this,
    );
  }
  put(endpoint, body, config) {
    return __checkpoint__.execute(
      "this.request",
      this.request,
      ["PUT", endpoint, { ...config, body }],
      this,
    );
  }
  delete(endpoint, config) {
    return __checkpoint__.execute(
      "this.request",
      this.request,
      ["DELETE", endpoint, config],
      this,
    );
  }
}
export { ApiClient };
console.log("Transformed code is executing!");

const client = new ApiClient("https://api.example.com");
client.get("/test", {}).then(console.log).catch(console.error);
