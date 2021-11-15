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
communication concept makes it possible to provide a simple implementation for a wide variety of applications.
