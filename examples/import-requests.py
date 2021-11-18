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
    import text from 'test';
    
    console.log(text)
    """
    await stream.write(IsolateRequest(script_schedule_message=ScheduleIsolateScriptMessage(
        kind=ScheduleIsolateScriptMessage.ScriptKind.MODULE,
        content=script
    )))

    async for resp in stream:
        if resp.HasField("script_resource_request"):
            msg = resp.script_resource_request
            if msg.kind == "console":
                print(msg.payload.decode("utf-8"))
            elif msg.kind == "module":
                module_name = msg.payload.decode("utf-8").split("/")[-1]
                if module_name == "test":
                    await stream.write(IsolateRequest(script_resource_response=IsolateScriptResourceResponseMessage(
                        nonce=msg.nonce,
                        payload="const test = 'this text has been imported'; export default test;".encode("utf-8")
                    )))
        else:
            print(resp)


asyncio.run(main())
