# Isolator (WIP)

An attempt to make an easy-to-use service that allows to run arbitary and untrusted JavaScript code. Communication
happens over a gRPC service which allows the client to schedule scripts and configure the service during runtime. It
internally uses V8 and sandboxes the code using V8 isolates.

To execute a script the client opens a bidirectional streaming connection to the service. Each of these connections is
associated with one V8 isolate (and one thread internally). The client can schedule multiple scripts into that isolate
over the connection and perform some other script-related actions. The script can use the provided JavaScript APIs to
send so-called "Resource Requests" over the connection to the client. The client can fulfill these requests and respond
to the request with a so-called "Resource Response" again over the connection.  
The client is therefore responsible for implementing necessary APIs to access external resources from the script. This
communication concept makes it possible to provide a simple and secure implementation for a wide variety of
applications.

## Security

Each execution environment is isolated using V8 isolates. This is the same technology that Google Chrome uses to isolate
browser tabs, web workers, etc. These isolates prevent the code from accessing any resources outside of the isolate (
even if they are in the same process).

The approach of Isolator is pretty similar to that of Cloudflare Workers. You can read more about it here:
https://developers.cloudflare.com/workers/learning/security-model

Now code that is completely isolated isn't very useful. (Schrödingers Cat paradox)
Isolator exposes a single API to communicate with the outside called Resource requests. The JS code can send resource
requests in the form of a string that can be used to distinguish between different request types and a bytestring that
can contain optional payload data. The JS code can optionally wait for a response for the resource request. This
approach gives potential attackers a very small attack surface.

V8 isolates are already very secure (if you trust Chrome to execute random code from the internet you can also trust
this)
and should prevent attackers from accessing any external resources if there is no zero-day vulnerability. But it's
always better to have multiple layers of security, so here are some measures that you can take to make it even more
secure:

### Additional security measures

- Use linux namespaces to prevent access to the filesystem and network for the Isolator process
- Use firewall rules to prevent any network traffic beside the incoming gRPC requests
- Run the Isolator process(es) in a separate VM or LXC
- Periodically draining and restarting the Isolator process(es)

## Defining Limits

### Thread Pool

Each Isolator instance can run up to 100 isolates in parallel by default. Each isolate acquires its own thread from a
thread pool and only frees it when the gRPC connection is closed. You can increase the thread pool size (and therefore
the count of isolates that can run in parallel) by setting the `THREAD_POOL_SIZE` environment variable.

### Heap Limits

The heap limits are used to define how much memory each isolate can consume. You can set two different heap limits in
the InitializeIsolate message:

- **soft_heap_limit**: The V8 isolate is terminated when this limited is reached. The isolate is still allowed to
  allocate more memory while the isolate is terminating to prevent the process for panicking.
- **hard_heap_limit**: In very rare cases the V8 isolate can not be terminated and will be able to continue allocating
  more memory. In this case the hard limit will kick in and disallow the isolate to allocate any more more memory. This
  usually causes the whole process to panic. (still better than allowing the isolate to allocate unlimited memory)

**The default (and custom) scripts that are loaded for each isolate at startup already consume 1 - 2 MB of heap. If the
initial scripts don't fit into the heap limit the process will panic when the first isolate is created.**

### Time Limits

The time limits are used to define how much time an isoalte can spend executing code. You can set two different time
limits in the InitializeIsolate message:

- **cpu_time_limit**: The count of milliseconds that the isolate is allowed to spend actually processing something. This
  does not include waiting for timers, resource requests, etc.
- **execution_time_limit**: The count of milliseconds that the isolate is allowed to spend executing. This is the total
  time it spends running scripts there were scheduled by the client.

Keep in mind that terminating the isolate can take a few milliseconds. So the actual time is usually `1-3 ms` longer.

## Resource Requests

Resource requests are used to access external resources from isolated the JavaScript code. The runtime exposes two
functions on the global `Isolator` objects to make resource requests:

```js
Isolator.makeResourceRequest('kind', 'payload');
Isolator.makeResourceRequestAndWait('kind', 'payload');
```

The promise returned by `makeResourceRequestAndWait` will only resolve if the client responds to the request. There are
a few default resource requests, and you can implement your own ones.

### Default resource requests:

- **console**: Sent for `console.log`, `console.warn` etc. The payload is the UTF-8 encoded string. Expects no response.
- **module**: Sent when an ESM module is imported using `import`. The payload is the UTF-8 encoded specifier for the
  module. Expects an UTF-8 encoded JSON string with the following format:
    ```json
    {
      "found": true,
      "content": "const test = 'this is a test'; export default test;"
    }
    ```

## Scaling

If you want to run a large number of isolates in parallel (1000+) it probably makes sense to run multiple Isolator
processes and balance the load between them.  
Load balancing can be achieved by using Nginx as a reverse proxy. The `least_conn` balancing algorithm would make the
most sense.

## TODO

- fix execution time limit to only count actual execution time
- implement max resource request count