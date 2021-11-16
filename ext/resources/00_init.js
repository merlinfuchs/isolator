"use strict";

((window) => {
    const core = window.Deno.core;

    async function makeResourceRequestWithResponse(kind, payload) {
        const result = await core.opAsync("op_resource_request_response", {kind, payload})
        return result.payload
    }

    async function makeResourceRequest(kind, payload) {
        await core.opAsync("op_resource_request", {kind, payload})
    }

    window.Isolator = {
        makeResourceRequestWithResponse,
        makeResourceRequest
    }
})(this)