# radicula-http

Synchronous HTTP transport and a JSON request facade. This is an independent
library root. It depends on no lock, Git, process, or other PerishLab product.

`wire::Client` sends GET, DELETE, POST, PATCH, and PUT under a caller-supplied
`Policy`: redirect limit, overall timeout, connect timeout, and response body
limit. HTTP status stays in the result. An unsupported method, a body on GET
or DELETE, a transport failure, and an unreadable body are different errors.
Collected response header values and the response body are held in zeroizing
buffers. The JSON facade also zeroizes the bytes it encodes for a request.
The public `Response` then stores ordinary header strings and a JSON value.
Callers' own strings are left as they were.

`Request`, `send`, `raw`, and `request` use the default policy: 10 redirects,
no overall timeout, no connect timeout, and a 10 MiB body limit. `send` returns
the JSON body for statuses 200 through 299 and a `Fault` for any other status.
`raw` and `request` return that status. `Fault` display uses a string
`message` field when the body has one, and its debug form omits the body.
`Request::using` sends the same JSON request through a caller-supplied client.

The library does not sign requests, classify a provider payload, retry, or
hold credentials. Those policies stay with the caller.
