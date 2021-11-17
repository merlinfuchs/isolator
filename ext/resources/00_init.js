"use strict";

((window) => {
    const bootstrap = window.__bootstrap;
    const core = window.Deno.core;
    const {
        ObjectAssign
    } = bootstrap.primordials;

    async function makeResourceRequestWithResponse(kind, payload) {
        const result = await core.opAsync("op_resource_request_response", {kind, payload})
        return result.payload
    }

    async function makeResourceRequest(kind, payload) {
        await core.opAsync("op_resource_request", {kind, payload})
    }

    ObjectAssign(bootstrap, {
        isolator: {
            makeResourceRequestWithResponse,
            makeResourceRequest
        }
    })
})(globalThis)