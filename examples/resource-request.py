import json

import grpc
import asyncio

from service_pb2_grpc import IsolatorStub
from service_pb2 import IsolateRequest, InitializeIsolateMessage, ScheduleIsolateScriptMessage


async def main():
    channel = grpc.aio.insecure_channel("localhost:50051")
    isolator = IsolatorStub(channel)

    stream = isolator.AcquireIsolate()

    print("send initialize")
    await stream.write(IsolateRequest(initialize_message=InitializeIsolateMessage()))

    print("send schedule")
    script = """
    Isolator.makeResourceRequestWithResponse('test', JSON.stringify({yeet: 5}))
    """
    await stream.write(IsolateRequest(schedule_message=ScheduleIsolateScriptMessage(content=script)))

    async for resp in stream:
        print(type(resp), resp)
        print(resp.script_resource_request)
        if hasattr(resp, "script_resource_request"):
            print(json.loads(resp.script_resource_request.payload))


asyncio.run(main())
