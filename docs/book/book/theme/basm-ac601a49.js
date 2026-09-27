// Highlighting for ```basm blocks. mdBook's highlight.js has already run by
// the time this loads, so register the language and highlight those blocks
// again.
(function () {
    if (typeof hljs === "undefined") {
        return;
    }

    hljs.registerLanguage("basm", function (hljs) {
        return {
            name: "BitterASM",
            aliases: ["bitterasm"],
            keywords: {
                keyword: "from import as in pub skip macro type struct enum const section",
                type: "int",
            },
            contains: [
                hljs.COMMENT("#", "$"),
                hljs.QUOTE_STRING_MODE,
                { className: "string", begin: /'(\\.|[^'\\])'/ },
                { className: "keyword", begin: /@[A-Za-z_]\w*/ },
                {
                    // Facets: `| syntax { ... }`, `| invariant ...`, ...
                    className: "built_in",
                    begin: /\|\s*(syntax|invariant|before|after|emits|to|from|allow|warn|deny|forbid|expect)\b/,
                },
                { className: "subst", begin: /`/, end: /`/ },
                { className: "number", begin: /\b(0x[0-9A-Fa-f_]+|0b[01_]+|\d[\d_]*)\b/ },
                { className: "title", begin: /^\s*[A-Za-z_]\w*:(?=\s*$)/ },
            ],
        };
    });

    var highlight = hljs.highlightElement || hljs.highlightBlock;
    document.querySelectorAll("code.language-basm").forEach(function (block) {
        block.textContent = block.textContent;
        block.classList.remove("hljs");
        highlight.call(hljs, block);
        block.classList.add("hljs");
    });
})();
