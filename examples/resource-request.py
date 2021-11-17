import json

import grpc
import asyncio

from service_pb2_grpc import IsolatorStub
from service_pb2 import IsolateRequest, InitializeIsolateMessage, ScheduleIsolateScriptMessage, \
    IsolateScriptResourceResponseMessage


async def main():
    channel = grpc.aio.insecure_channel("localhost:50051")
    isolator = IsolatorStub(channel)

    stream = isolator.AcquireIsolate()

    await stream.write(IsolateRequest(initialize_message=InitializeIsolateMessage()))

    script = """
    async function test() {
        resp = await Isolator.makeResourceRequestWithResponse('read_file', 'test.txt')
        let t = new Date();
        print(JSON.stringify(t))
    }
    
    test()
    """
    await stream.write(IsolateRequest(script_schedule_message=ScheduleIsolateScriptMessage(content=script)))

    async for resp in stream:
        print(resp)
        if resp.HasField("script_resource_request"):
            msg = resp.script_resource_request
            if msg.kind == "read_file":
                filename = msg.payload.decode("utf-8")
                with open(filename, "rb") as fp:
                    await stream.write(IsolateRequest(script_resource_response=IsolateScriptResourceResponseMessage(
                        resource_id=msg.resource_id,
                        payload=fp.read()
                    )))


asyncio.run(main())
