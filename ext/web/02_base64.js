"use strict";

((window) => {
    const webidl = window.__bootstrap.webidl;
    const {
        ArrayPrototypeMap,
        StringPrototypeCharCodeAt,
        ArrayPrototypeJoin,
        StringFromCharCode,
        TypedArrayFrom,
        Uint8Array,
    } = window.__bootstrap.primordials;
    const {DOMException} = window.__bootstrap.domException;
    const {
        forgivingBase64Encode,
        forgivingBase64Decode,
    } = window.__bootstrap.infra;

    function atob(data) {
        data = webidl.converters.DOMString(data, {
            prefix: "Failed to execute 'atob'",
            context: "Argument 1",
        });

        const uint8Array = forgivingBase64Decode(data);
        const result = ArrayPrototypeMap(
            [...uint8Array],
            (byte) => StringFromCharCode(byte),
        );
        return ArrayPrototypeJoin(result, "");
    }

    function btoa(data) {
        const prefix = "Failed to execute 'btoa'";
        webidl.requiredArguments(arguments.length, 1, {prefix});
        data = webidl.converters.DOMString(data, {
            prefix,
            context: "Argument 1",
        });
        const byteArray = ArrayPrototypeMap([...data], (char) => {
            const charCode = StringPrototypeCharCodeAt(char, 0);
            if (charCode > 0xff) {
                throw new DOMException(
                    "The string to be encoded contains characters outside of the Latin1 range.",
                    "InvalidCharacterError",
                );
            }
            return charCode;
        });
        return forgivingBase64Encode(TypedArrayFrom(Uint8Array, byteArray));
    }

    window.__bootstrap.base64 = {
        atob,
        btoa,
    };
})(globalThis)