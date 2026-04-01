# Mojo 🔥: Powerful CPU+GPU Programming
Source: https://www.modular.com/mojo

One language, any hardware. Systems-level performance. Pythonic syntax. 

Mojo unifies high-level AI development with low-level systems programming. Write once, deploy everywhere - from CPUs to GPUs - without vendor lock-in. 

Start building now! 

Mojo’s roadmap 

Mojo highlights 

GPU Programming 

Python Interop 

Metaprogramming 

# GPU kernel performing parallel vector addition. 

def vector_add ( 
result : LayoutTensor [ float_dtype, layout, MutAnyOrigin ] , 
a : LayoutTensor [ float_dtype, layout, MutAnyOrigin ] , 
b : LayoutTensor [ float_dtype, layout, MutAnyOrigin ] , 
) : 

i = Int ( global_idx . x ) 
if i < size : 
result [ i ] = a [ i ] + b [ i ] 

# SIMD-vectorized kernel squaring array elements in place. 

def mojo_square_array ( array_obj : PythonObject ) raises : 
comptime simd_width = simd_width_of [ DType.int64 ] ( ) 
ptr = array_obj . ctypes . data . unsafe_get_as_pointer [ DType.int64 ] ( ) 

def pow [ width : Int ] ( i : Int ) unified { mut ptr } : 
elem = ptr . load [ width = width ] ( i ) 
ptr . store [ width = width ] ( i , elem * elem ) 

vectorize [ simd_width ] ( len ( array_obj ) , pow ) 

# Hardware-dispatched vector addition for CPU or GPU. 

struct VectorAddition : 
@ staticmethod 
def execute [ 
target : StaticString, 
] ( 
output : OutputTensor [ rank = 1 , ... ] , 
lhs : InputTensor [ dtype = output . dtype , rank = output . rank , ... ] , 
rhs : InputTensor [ dtype = output . dtype , rank = output . rank , ... ] , 
ctx : DeviceContextPtr, 
) raises : 
comptime if is_cpu ( target ) : 
vector_addition_cpu ( output , lhs , rhs , ctx ) 
elif is_gpu ( target ) : 
vector_addition_gpu ( output , lhs , rhs , ctx ) 
else : 
raise Error ( "No known target:" , target ) 

Why we built Mojo? 

Vendor lock-in is expensive 

You're forced to choose: NVIDIA's CUDA, AMD's ROCm, or Intel's oneAPI. Rewrite everything when you switch vendors. Your code becomes a hostage to hardware politics. 

The two-language tax 

Prototype in Python. Rewrite in C++ for production. Debug across language boundaries. Your team splits into 'researchers' and 'engineers' - neither can work on the full stack. 

Python hits a wall 

Python is 1000x too slow for production AI. The GIL blocks true parallelism. Can't access GPUs directly. Every optimization means dropping into C extensions. Simplicity becomes a liability at scale. 

Toolchain chaos 

PyTorch for training. TensorRT for inference. vLLM for serving. Each tool has its own bugs, limitations, and learning curve. Integration nightmares multiply with every component. 

Memory bugs in production 

C++ gives you footguns by default. Race conditions in parallel code. Memory leaks that OOM your servers. Segfaults in production at 3 AM. 

Developer experience ignored 

30-minute build times. Cryptic template errors. Debuggers that can't inspect GPU state. Profilers that lie about performance. Modern developers deserve tools that accelerate, not frustrate. 

Why should I use Mojo? 
Easier 

GPU Programming Made Easy 

Traditionally, writing custom GPU code means diving into CUDA, managing memory, and compiling separate device code. Mojo simplifies the whole experience while unlocking top-tier performance on NVIDIA and AMD GPUs. 

Get Started With GPUs 

comptime for n_mma in range ( num_n_mmas ) : 
comptime mma_id = n_mma * num_m_mmas + m_mma 
var mask_frag_row = mask_warp_row + m_mma * MMA_M 
var mask_frag_col = mask_warp_col + n_mma * MMA_N 
comptime if is_nvidia_gpu () : 
mask_frag_row += lane // ( MMA_N // p_frag_simdwidth ) 
mask_frag_col += (lane * p_frag_simdwidth) % MMA_N 
elif is_amd_gpu () : 
mask_frag_row += (lane // MMA_N) * p_frag_simdwidth 
mask_frag_col += lane % MMA_N 

Performant 

Bare metal performance on any GPU 

Get raw GPU performance without complex toolchains. Mojo makes it easy to write high-performance kernels with intuitive syntax, zero boilerplate, and native support for NVIDIA, AMD, and more. 

GPU Fundamentals 

comptime for i in range ( K ) : 
var reduced = top_k_sram[tid] 
comptime limit = log2_floor ( WARP_SIZE ) 

comptime for j in reversed ( range ( limit )) : 
comptime offset = 1 << j 
var shuffled = TopKElement ( 
warp.shuffle_down ( reduced.idx, offset ) , 
warp.shuffle_down ( reduced.val, offset ) , 
) 
reduced = max ( reduced, shuffled ) 

barrier () 

Interoperable 

Use Mojo to extend python 

Mojo interoperates natively with Python so you can speed up bottlenecks without rewriting everything. Start with one function, scale as needed—Mojo fits into your codebase 

Intro to Python Interop 

if __name__ == "__main__" : 
# Calling into a Mojo `passthrough` function from Python: 
result = hello_mojo.passthrough ( "Hello" ) 
print ( result ) 

fn passthrough ( value : PythonObject ) raises -> PythonObject: 
"""A very basic function illustrating passing values to and from Mojo.""" 
return value + " world from Mojo" 

Community 

Build with us in the open to create the future of AI 

Mojo has more than  750K+ lines of open-source code with an active community of 50K+ members. We're actively working to open even more to build a transparent, developer-first foundation for the future of AI infrastructure. 

View Open Kernel Repo 

750k 

lines of open-source code 

MOJO + MAX 

Write GPU Kernels with MAX 

Traditionally, writing custom GPU code means diving into CUDA, managing memory, and compiling separate device code. Mojo simplifies the whole experience while unlocking top-tier performance on NVIDIA and AMD GPUs. 

Read more 

@compiler.register ( "mo.sub" ) 
struct Sub : 
@ staticmethod 
def execute [ target: StaticString, _trace_name: StaticString ]( 
z : FusedOutputTensor, 
x : FusedInputTensor, 
y : FusedInputTensor, 
ctx : DeviceContextPtr, 
) capturing raises : 
@ parameter 
@ always_inline 
def func [ width: Int ] (idx: IndexList[z.rank]) -> SIMD[z.dtype, width]: 
var lhs = rebind[SIMD[z.dtype, width]](x._fused_load[width](idx)) 
var rhs = rebind[SIMD[z.dtype, width]](y._fused_load[width](idx)) 
return lhs - rhs 

foreach [ 
func, 
target=target, 
_trace_name=_trace_name, 
] (z, ctx) 

Interoperable 

Powering Breakthroughs in Production AI 

Top AI teams use Mojo to turn ideas into optimized, low-level GPU code. From Inworld’s custom logic to Qwerky’s memory-efficient Mamba, Mojo delivers where performance meets creativity. 

Inworld Case Study 

Qwerky Case Study 

Inworld 
Inworld used Mojo to define high-efficiency custom kernels to create things like a tailored silence-detection kernel that runs directly on the GPU. 

Qwerky 
Mojo enables Qwerky to compile custom GPU kernels accelerating Mamba's linear-time complexity for conversation history 

Performant 

World-Class Tools, Out of the Box 

Mojo ships with a great VSCode debugger and works with dev tools like Cursor and Claude. Mojo makes modern dev workflows feel seamless. 

Get VSCode Extension 

Mojo learns from 

C++ 

Python 

Rust 

Zig 

What Mojo keeps from C++ 

Zero cost abstractions 

Metaprogramming power 
Turing complete: can build a compiler in templates 

Low level hardware control 
Inline asm, intrinsics, zero dependencies 

Unified host/device language 

What Mojo improves about C++ 

Slow compile times 

Template error messages 

Limited metaprogramming 
...and that templates != normal code 

Not MLIR-native 

What Mojo keeps from Python 

Minimal boilerplate 

Easy-to-read syntax 

Interoperability with the massive Python ecosystem 

What Mojo improves about Python 

Performance 

Memory usage 

Device portability 

What Mojo keeps from Rust 

Memory safety through borrow checker 

Systems language performance 

What Mojo improves about Rust 

More flexible ownership semantics 

Easier to learn 

More readable syntax 

What Mojo keeps from Zig 

Compile-time metaprogramming 

Systems language performance 

What Mojo improves about Zig 

Memory safety 

More readable syntax 

Get started with Mojo 

Start using Mojo 
( FREE ) 

Install Mojo and get up and running in minutes. A simple install, familiar tooling, and clear docs make it easy to start writing code immediately. 

Install Mojo🔥 

Mojo Quickstart 

Easy ways to get started 

Not sure where to start?  These examples below give you a few simple entry points into Mojo. 

Mojo Manual 
Write a simple GPU program and learn the basics. 

GPU Puzzles 
Practice GPU programming with guided puzzles. 

Python Interoperability 
Read and write Mojo using familiar Python syntax. 

Popular Mojo Tech Talks 

Next-Gen GPU Programming 

1:15:56 

Kernel Programming and Mojo 

52:51 

GPU Programming Workshop 

11:36 

“Mojo has Python feel, systems speed. Clean syntax, blazing performance.” 

Explore the world of high-performance computing through an illustrated comic. A fresh, fun take—whether you're new or experienced. 

Read the comic 

Developer Approved 

impressed 
justin_76273 

“The more I benchmark, the more impressed I am with the MAX Engine.” 

12x faster without even trying 
svpino 

“Mojo destroys Python in speed. 12x faster without even trying. The future is bright!” 

completely different ballgame 
scrumtuous 

“What @modular is doing with Mojo and the MaxPlatform is a completely different ballgame.” 

The future is bright! 
mytechnotalent 

Mojo destroys Python in speed. 12x faster without even trying. The future is bright! 

amazing achievements 
Eprahim 

“I'm excited, you're excited, everyone is excited to see what's new in Mojo and MAX and the amazing achievements of the team at Modular.” 

pure iteration power 
Jayesh 

"This is about unlocking freedom for devs like me, no more vendor traps or rewrites, just pure iteration power. As someone working on challenging ML problems, this is a big thing." 

works across the stack 
scrumtuous 

“Mojo can replace the C programs too. It works across the stack. It’s not glue code. It’s the whole ecosystem.” 

one language all the way through 
fnands 

“Tired of the two language problem. I have one foot in the ML world and one foot in the geospatial world, and both struggle with the 'two-language' problem. Having Mojo - as one language all the way through is be awesome.” 

performance is insane 
drdude81 

“I tried MAX builds last night, impressive indeed. I couldn't believe what I was seeing... performance is insane.” 

high performance code 
jeremyphoward 

"Mojo is Python++. It will be, when complete, a strict superset of the Python language. But it also has additional functionality so we can write high performance code that takes advantage of modern accelerators." 

actually flies on the GPU 
Sanika 

"after wrestling with CUDA drivers for years, it felt surprisingly… smooth. No, really: for once I wasn’t battling obscure libstdc++ errors at midnight or re-compiling kernels to coax out speed. Instead, I got a peek at writing almost-Pythonic code that compiles down to something that actually flies on the GPU." 

Community is incredible 
benny.n 

“The Community is incredible and so supportive. It’s awesome to be part of.” 

huge increase in performance 
Aydyn 

"C is known for being as fast as assembly, but when we implemented the same logic on Mojo and used some of the out-of-the-box features, it showed a huge increase in performance... It was amazing." 

surest bet for longterm 
pagilgukey 

“Mojo and the MAX Graph API are the surest bet for longterm multi-arch future-substrate NN compilation” 

impressive speed 
Adalseno 

"It worked like a charm, with impressive speed. Now my version is about twice as fast as Julia's (7 ms vs. 12 ms for a 10 million vector; 7 ms on the playground. I guess on my computer, it might be even faster). Amazing." 

easy to optimize 
dorjeduck 

“It’s fast which is awesome. And it’s easy. It’s not CUDA programming...easy to optimize.” 

potential to take over 
svpino 

“A few weeks ago, I started learning Mojo 🔥 and MAX. Mojo has the potential to take over AI development. It's Python++. Simple to learn, and extremely fast.” 

was a breeze! 
NL 

“Max installation on Mac M2 and running llama3 in (q6_k and q4_k) was a breeze! Thank you Modular team!” 

very excited 
strangemonad 

“I'm very excited to see this coming together and what it represents, not just for MAX, but my hope for what it could also mean for the broader ecosystem that mojo could interact with.” 

feeling of superpowers 
Aydyn 

"Mojo gives me the feeling of superpowers. I did not expect it to outperform a well-known solution like llama.cpp." 

Show more quotes

---

# GitHub - modular/modular: The Modular Platform (includes MAX & Mojo) · GitHub
Source: https://github.com/modularml/mojo

About Modular | Get started | API docs | Contributing 
| Changelog | MAX Model Development 

🤝 Join our monthly community meetings : the next
meeting is scheduled for Monday, March 23rd at 10am PT . 

Modular Platform 

A unified platform for AI development and deployment, including MAX 🧑‍🚀 and
Mojo 🔥. 

The Modular Platform is an open and fully-integrated suite of AI libraries
and tools that accelerates model serving and scales GenAI deployments. It
abstracts away hardware complexity so you can run the most popular open
models with industry-leading GPU and CPU performance without any code changes. 

Get started 

You don't need to clone this repo. 

You can install Modular as a pip or conda package and then start an
OpenAI-compatible endpoint with a model of your choice. 

To get started with the Modular Platform and serve a model using the MAX
framework, see the quickstart guide . 

Note 
Nightly vs. stable releases 
If you cloned the repo and want a stable release, run
git checkout modular/vX.X to match the version.
The main branch tracks nightly builds, while the stable branch matches
the latest released version. 

After your model endpoint is up and running, you can start sending the model
inference requests using
our OpenAI-compatible REST API . 

Explore all the models you can deploy with Modular in our
Model Library . 

Deploy our container 

The MAX container is our Kubernetes-compatible Docker container for convenient
deployment, which uses the MAX framework's built-in inference server. We have
separate containers for NVIDIA and AMD GPU environments, and a unified container
that works with both. 

For example, you can start a container for an NVIDIA GPU with this command: 

docker run --gpus=1 \
-v ~ /.cache/huggingface:/root/.cache/huggingface \
-p 8000:8000 \
modular/max-nvidia-full:latest \
--model-path google/gemma-3-27b-it 

For more information, see our MAX container
docs or the Modular Docker Hub
repository . 

About the repo 

We're constantly open-sourcing more of the Modular Platform and you can find
all of it in here. As of May, 2025, this repo includes over 450,000 lines of
code from over 6000 contributors, providing developers with production-grade
reference implementations and tools to extend the Modular Platform with new
algorithms, operations, and hardware targets. 

It's quite likely the world's largest repository of open source CPU and GPU
kernels ! 

Highlights include: 

Mojo standard library: /mojo/stdlib 

MAX GPU and CPU kernels: /max/kernels (Mojo kernels) 

MAX inference server: /max/python/max/serve 
(OpenAI-compatible endpoint) 

MAX model pipelines: /max/python/max/pipelines 
(Python-based graphs) 

Code examples: /max/examples + /mojo/examples 

This repo has two major branches: 

The main branch, which is
in sync with the nightly build and subject to new bugs. Use this branch for
contributions , or if you installed the nightly
build . 

The stable branch, which
is in sync with the last stable released version of Mojo. Use the examples in
here if you installed the stable
build . 

Contribute 

We accept contributions to the Mojo standard library , MAX AI
kernels , MAX model
architectures , code examples, Mojo
docs, and more. 

First, please read the Contribution Guide , and then refer
to the following documentation about how to develop in the repo: 

/max/docs : Docs for developers working in the MAX framework codebase. 

/mojo/stdlib/docs : Docs for developers working in the
Mojo standard library. 

We also welcome your bug reports. If you have a bug, please file an issue
here . 

News & Announcements 

[2026/2] We announced that BentoML is joining Modular .
We are committed to building in the open and will be extending our support
of open source AI with Bento's own open project .
Read the answers in our February 2026 AMA to learn more
about our plans. 

[2026/1] Modular Platform 26.1 graduates the MAX Python API out of
experimental with PyTorch-like eager mode and model.compile() for production,
stabilizes the MAX LLM Book, and expands Apple silicon GPU support. Mojo gains
compile-time reflection, linear types, typed errors, and improved error messages
as it progresses toward 1.0. 

[2025/12] The Path to Mojo 1.0 was officially announced
with a planned release in H1 2026 and tons of details on what to expect. 

[2025/12] We hosted our Inside the MAX Framework Meetup 
reintroducing the MAX framework and taking the community through upcoming
changes. 

[2025/11] Modular Platform 25.7 provides a fully open MAX Python
API, expanded hardware support for NVIDIA Grace superchips, improved Mojo GPU
programming experience, and much more. 

[2025/11] We met with the community at
PyTorch 2025 + the LLVM Developers' Meeting to solicit
community input into how the Modular platform can reduce fragmentation and
provide a unified AI stack. 

[2025/09] Modular raises $250M to scale AI's unified compute
layer, bringing Modular's total raise to $380M at a $1.6B valuation. 

[2025/09] Modular Platform 25.6 delivers a unified compute layer
spanning from laptops to datacenter GPUs, with industry-leading throughput on
NVIDIA Blackwell (B200) and AMD MI355X. 

[2025/08] Modular Platform 25.5 introduces Large Scale Batch
Inference through a partnership with SF Compute + open source launch of the
MAX Graph API and more. 

[2025/08] We hosted our Los Altos Meetup featuring talks from
Chris Lattner on democratizing AI compute and Inworld AI on production voice AI. 

[2025/06] AMD partnership announced — Modular Platform now generally
available across AMD's MI300 and MI325 GPU portfolio. 

[2025/06] Modular Hack Weekend brought developers together
to build custom kernels, model architectures, and PyTorch custom ops with
Mojo and MAX. 

[2025/05] Over 100 engineers gathered at AGI House for our first
GPU Kernel Hackathon , featuring talks from Modular and
Anthropic engineers. 

Community & Events 

We host regular meetups digitally and around the world. During these meetups
we share updates from the Modular team, feature community contributions, and
invite guest speakers to share their expertise, as well as answer community
questions. 

Join us! 

Channel 
Link 

💬 Discord 
discord.gg/modular 

💬 Forum 
forum.modular.com 

📅 Meetup Group 
meetup.com/modular-meetup-group 

🎥 Community Meetings 
Upcoming community calls 

Upcoming events will be posted on our Meetup page and
Discord . Community meeting recordings will be posted on our
YouTube . 

Contact us 

If you'd like to chat with the team and other community members, please send a
message to our Discord channel and our
forum board . 

License 

This repository and its contributions are licensed under the Apache License
v2.0 with LLVM Exceptions (see the LLVM License ).
Modular, MAX and Mojo usage and distribution are licensed under the
Modular Community License . 

Third party licenses 

You are entirely responsible for checking and validating the licenses of
third parties (i.e. Huggingface) for related software and libraries that are downloaded. 

Thanks to our contributors

---

# Mojo Manual | Modular
Source: https://docs.modular.com/mojo/

Welcome to the Mojo Manual, your complete guide to the Mojo🔥 programming language! 

Combined with the Mojo API reference , this documentation provides
everything you need to write high-performance Mojo code for CPUs and GPUs. If
you see anything that can be improved, please file an
issue 
or send a pull request for the docs on
GitHub . 

Get started with a tutorial 
About Mojo ​ 

Mojo is a systems programming language specifically designed for
high-performance AI infrastructure and heterogeneous hardware. Its Pythonic
syntax makes it easy for Python programmers to learn and it fully integrates
the existing Python ecosystem, including its wealth of AI and machine-learning
libraries. 

It's the first programming language built from the ground-up using MLIR—a
modern compiler infrastructure for heterogeneous hardware, from CPUs to GPUs
and other AI ASICs. That means you can use one language to write all your code,
from high-level AI applications all the way down to low-level GPU kernels,
without using any hardware-specific libraries (such as CUDA and ROCm). 

Learn more about it in the Mojo vision doc. 

Key features ​ 

Python syntax & interop : Mojo adopts (and extends) Python's syntax and
integrates with existing Python code. Mojo's interoperability works in both
directions, so you can import Python libraries into Mojo and create Mojo
bindings to call from Python. Read about Python
interop . 

Struct-based types : All data types—including basic types such as String 
and Int —are defined as structs. No types are built into the language itself.
That means you can define your own types that have all the same
capabilities as the standard library types. Read about
structs . 

Zero-cost traits : Mojo's trait system solves the problem of static typing
by letting you define a shared set of behaviors that types (structs) can
implement. It allows you to write functions that depend on traits rather than
specific types, similar to interfaces in Java or protocols in Swift, except
with compile-time type checking and no run-time performance cost. Read about
traits . 

Value semantics : Mojo supports both value and reference semantics, but
generally defaults to value semantics. With value semantics, each copy is
independent—modifying one copy won't affect another. With reference
semantics, multiple variables can point to the same instance (sometimes called
an object), so changes made through one variable are visible through all others.
Mojo-native types predominantly use value semantics, which prevents multiple
variables from unexpectedly sharing the same data. Read about value
semantics . 

Value ownership : Mojo's ownership system ensures that only one variable
"owns" a specific value at a given time—such that Mojo can safely deallocate
the value when the owner's lifetime ends—while still allowing you to share
references to the value. This provides safety from errors such as
use-after-free, double-free, and memory leaks without the overhead cost of a
garbage collector. Read about ownership . 

Compile-time metaprogramming : Mojo's parameterization system enables
powerful metaprogramming in which the compiler generates a unique version of a
type or function based on parameter values, similar to C++ templates, but more
intuitive.
Read about parameterization . 

Hardware portability : Mojo is designed from the ground up to support
heterogeneous hardware—the Mojo compiler makes no assumptions about whether
your code is written for CPUs, GPUs, or something else. Instead, hardware
behaviors are handled by Mojo libraries, as demonstrated by types such as
SIMD that allows you to write vectorized code for CPUs, and the gpu 
package that enables hardware-agnostic GPU programming. Read about GPU
programming . 

Get started ​ 

Get started with Mojo Install Mojo and learn the language basics by building a complete Mojo program 

Get started with GPU programming Learn the basics of GPU programming with Mojo 

GPU Puzzles Learn to program GPUs in Mojo by solving increasingly complex challenges 

Tip: To use AI coding assistants with Mojo, see our guide for
using Mojo AI skills . 

More resources ​ 

Mojo API reference Mojo standard library and other references 

Code examples Browse a wide range of Mojo code examples on GitHub 

Community Chat with us and the community in our forum and Discord channels 

Was this page helpful? 

Thank you! We'll create more content like this. Thank you for helping us improve! 😔 What went wrong? 
Some code doesn’t work 

It includes inaccurate information 

It's missing information I need 

It was difficult to understand 

Other 

Submit