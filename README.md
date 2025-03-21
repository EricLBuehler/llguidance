# Low-level Guidance (llguidance)

<p align="center">
    <img src="https://github.com/guidance-ai/jsonschemabench/raw/main/maskbench/plots/hero.png" width="700">
    <br/>
    <em>Performance results from <a href ="https://github.com/guidance-ai/jsonschemabench/tree/main/maskbench">MaskBench</a></em>
</p>

This library implements constrained decoding (also called constrained sampling or
structured outputs) for Large Langauge Models (LLMs).
It can enforce arbitrary context-free grammar on the output of LLM
and is fast - on the order of 50μs of CPU time per token
(for 128k tokenizer) with negligible startup costs.

Following grammar formats are supported:
- [a large subset](./docs/json_schema.md) of JSON schemas
- regular expressions
- context-free grammars in a [variation of Lark format](./docs/syntax.md);
  with embedded JSON schemas and regular expressions
- `llguidance` - [internal (JSON-based) format](./parser/src/api.rs);
  slowly being deprecated in favor of the Lark-like format

The internal format is most powerful (though Lark-like format is catching up, and there are plans to convert the libraries to use it) and can be generated by the following libraries:
- [Guidance](https://github.com/guidance-ai/guidance) (Python)
- [guidance.ts](https://github.com/mmoskal/guidance-ts) (TypeScript)
- hopefully more to come!

The library can be used from:
- [Rust](./parser/README.md), [sample](./sample_parser/src/minimal.rs)
- [C and C++](./parser/llguidance.h), [sample](./c_sample/c_sample.cpp)
- [Python](./python/llguidance/_lib.pyi)

## Integrations

The library is currently integrated in:
- [Guidance](https://github.com/guidance-ai/guidance) - library for interacting with LLMs
- [llama.cpp](https://github.com/ggerganov/llama.cpp/pull/10224) - 
  available via `-DLLAMA_LLGUIDANCE=ON` option for `cmake`;
  llama.cpp can be also used Guidance Python package
- [SGLang](https://github.com/sgl-project/sglang/pull/3298) -
  use `--grammar-backend llguidance`; when passing Lark grammar make
  sure to prefix them with `%llguidance {}`, just as in llama.cpp
- vLLM - [merged V0 PR](https://github.com/vllm-project/vllm/pull/14589) and [pending V1 PR](https://github.com/vllm-project/vllm/pull/14779)
- [LLGTRT](https://github.com/guidance-ai/llgtrt) - OpenAI-compatible REST server using NVIDIA's [TensorRT-LLM](https://github.com/NVIDIA/TensorRT-LLM)
- [mistral.rs](https://github.com/EricLBuehler/mistral.rs/pull/899)

The integration is ongoing in:
- onnxruntime-genai - [draft PR](https://github.com/microsoft/onnxruntime-genai/pull/1038)
- Chromium - [ongoing PR](https://chromium-review.googlesource.com/c/chromium/src/+/6232561)

## Technical details

Given a context-free grammar, a tokenizer, and a prefix of tokens, llguidance computes a token mask - a set of tokens from the tokenizer - that, when added to the current token prefix, can lead to a valid string in the language defined by the grammar. Mask computation takes approximately 50μs of single-core CPU time for a tokenizer with 128k tokens. While this timing depends on the exact grammar, it holds, for example, for grammars derived from JSON schemas. There is no significant startup cost.

The library implements a context-free grammar parser using Earley’s algorithm on top of a lexer based on [derivatives of regular expressions](https://github.com/microsoft/derivre). Mask computation is achieved by traversing the [prefix tree (trie)](./docs/toktrie.md) of all possible tokens, leveraging [highly optimized](./docs/optimizations.md) code.

Grammars can be also used to speed up decode via [fast-forward tokens](./docs/fast_forward.md).

### Comparison and performance

See [MaskBench](https://github.com/guidance-ai/jsonschemabench/tree/main/maskbench) in
[JSON Schema Bench](https://github.com/guidance-ai/jsonschemabench) for detailed performance comparisons.

[LM-format-enforcer](https://github.com/noamgat/lm-format-enforcer) and [llama.cpp grammars](https://github.com/ggerganov/llama.cpp/blob/master/grammars/README.md) are similar to llguidance in that they dynamically build token masks for every step of the decoding process. Both are significantly slower - the former due to clean Python code and the latter due to the lack of a lexer and use of a backtracking parser, which, while elegant, is inefficient.

[Outlines](https://github.com/dottxt-ai/outlines) builds an automaton from constraints and then pre-computes token masks for all automaton states, potentially making sampling fast but inherently limiting constraint complexity and introducing significant startup cost and memory overhead. Llguidance computes token masks on the fly and has essentially no startup cost. The lexer’s automata in llguidance are built lazily and are typically much smaller, as the context-free grammar imposes the top-level structure.

[XGrammar](https://github.com/mlc-ai/xgrammar) follows an approach similar to llama.cpp (explicit stack-based, character-level parser) with additional pre-computation of certain token masks, similar to Outlines. The pre-computation often runs into seconds, and sometimes minutes. If the pre-computation works well for a given input, the masks are computed quickly (under 8μs in half of masks we tested), however if it doesn't fit the particular input, 
the mask computation times can run to tens or hundreds of milliseconds.

In llguidance, the full mask computation for a typical JSON schema takes about 1.5ms (for 128k tokenizer).
However, very often the ["slicer" optimization](./docs/optimizations.md#slicer-optimization) applies,
and thus the avarage mask computation in [JSON Schema Bench](https://github.com/guidance-ai/jsonschemabench)
(2.5M tokens, 10k schemas) is under 50μs,
with less than 1% of masks taking longer than 1ms,
and 0.001% taking longer than 10ms (but still shorter than 30ms).
The optimization doesn't involve any significant pre-computation.

Thus, with 16 cores and a 10ms forward pass, llguidance can handle batch sizes up to 3200 without slowing down the model. (Note that a 10ms forward pass for small batch sizes typically increases to 20ms+ for batch sizes of 100-200.)

## Building

- [install rust](https://www.rust-lang.org/tools/install); 1.75 or later

If you just need the C or Rust library (`llguidance`), 
check the [parser](./parser/README.md) directory.

For Python bindings:

- install python 3.9 or later; very likely you'll need a virtual env/conda
- run `./scripts/install-deps.sh`
- to build and after any changes, run `./scripts/test-guidance.sh`

This builds the Python bindings for the library and runs the tests
(which mostly live in the Guidance repo - it will clone it).

## Contributing

This project welcomes contributions and suggestions. Most contributions require you to agree to a
Contributor License Agreement (CLA) declaring that you have the right to, and actually do, grant us
the rights to use your contribution. For details, visit https://cla.opensource.microsoft.com.

When you submit a pull request, a CLA bot will automatically determine whether you need to provide
a CLA and decorate the PR appropriately (e.g., status check, comment). Simply follow the instructions
provided by the bot. You will only need to do this once across all repos using our CLA.

This project has adopted the [Microsoft Open Source Code of Conduct](https://opensource.microsoft.com/codeofconduct/).
For more information see the [Code of Conduct FAQ](https://opensource.microsoft.com/codeofconduct/faq/) or
contact [opencode@microsoft.com](mailto:opencode@microsoft.com) with any additional questions or comments.

## Trademarks

This project may contain trademarks or logos for projects, products, or services. Authorized use of Microsoft
trademarks or logos is subject to and must follow
[Microsoft's Trademark & Brand Guidelines](https://www.microsoft.com/en-us/legal/intellectualproperty/trademarks/usage/general).
Use of Microsoft trademarks or logos in modified versions of this project must not cause confusion or imply Microsoft sponsorship.
Any use of third-party trademarks or logos are subject to those third-party's policies.
