# WebGPU
Source: https://www.w3.org/TR/webgpu/

1. Introduction 

This section is non-normative. 

Graphics Processing Units , or GPUs for short,
have been essential in enabling rich rendering and computational applications in personal computing.
WebGPU is an API that exposes the capabilities of GPU hardware for the Web.
The API is designed from the ground up to efficiently map to (post-2014) native GPU APIs.
WebGPU is not related to WebGL and does not explicitly target OpenGL ES. 

WebGPU sees physical GPU hardware as GPUAdapter s. It provides a connection to an adapter via
GPUDevice , which manages resources, and the device’s GPUQueue s, which execute commands.
GPUDevice may have its own memory with high-speed access to the processing units.
GPUBuffer and GPUTexture are the physical resources backed by GPU memory.
GPUCommandBuffer and GPURenderBundle are containers for user-recorded commands.
GPUShaderModule contains shader code. The other resources,
such as GPUSampler or GPUBindGroup , configure the way physical resources are used by the GPU. 

GPUs execute commands encoded in GPUCommandBuffer s by feeding data through a pipeline ,
which is a mix of fixed-function and programmable stages. Programmable stages execute
shaders , which are special programs designed to run on GPU hardware.
Most of the state of a pipeline is defined by
a GPURenderPipeline or a GPUComputePipeline object. The state not included
in these pipeline objects is set during encoding with commands,
such as beginRenderPass() or setBlendConstant() . 

2. Malicious use considerations 

This section is non-normative. It describes the risks associated with exposing this API on the Web. 

2.1. Security Considerations 

The security requirements for WebGPU are the same as ever for the web, and are likewise non-negotiable.
The general approach is strictly validating all the commands before they reach GPU,
ensuring that a page can only work with its own data. 

2.1.1. CPU-based undefined behavior 

A WebGPU implementation translates the workloads issued by the user into API commands specific
to the target platform. Native APIs specify the valid usage for the commands
(for example, see vkCreateDescriptorSetLayout )
and generally don’t guarantee any outcome if the valid usage rules are not followed.
This is called "undefined behavior", and it can be exploited by an attacker to access memory
they don’t own, or force the driver to execute arbitrary code. 

In order to disallow insecure usage, the range of allowed WebGPU behaviors is defined for any input.
An implementation has to validate all the input from the user and only reach the driver
with the valid workloads. This document specifies all the error conditions and handling semantics.
For example, specifying the same buffer with intersecting ranges in both "source" and "destination"
of copyBufferToBuffer() results in GPUCommandEncoder 
generating an error, and no other operation occurring. 

See § 22 Errors & Debugging for more information about error handling. 

2.1.2. GPU-based undefined behavior 

WebGPU shader s are executed by the compute units inside GPU hardware. In native APIs,
some of the shader instructions may result in undefined behavior on the GPU.
In order to address that, the shader instruction set and its defined behaviors are
strictly defined by WebGPU. When a shader is provided to createShaderModule() ,
the WebGPU implementation has to validate it
before doing any translation (to platform-specific shaders) or transformation passes. 

2.1.3. Uninitialized data 

Generally, allocating new memory may expose the leftover data of other applications running on the system.
In order to address that, WebGPU conceptually initializes all the resources to zero, although in practice
an implementation may skip this step if it sees the developer initializing the contents manually.
This includes variables and shared workgroup memory inside shaders. 

The precise mechanism of clearing the workgroup memory can differ between platforms.
If the native API does not provide facilities to clear it, the WebGPU implementation transforms the compute
shader to first do a clear across all invocations, synchronize them, and continue executing developer’s code. 

NOTE: 

The initialization status of a resource used in a queue operation can only be known when the
operation is enqueued (not when it is encoded into a command buffer, for example). Therefore,
some implementations will require an unoptimized late-clear at enqueue time (e.g. clearing a
texture, rather than changing GPULoadOp "load" to "clear" ).

As a result, all implementations should issue a developer console warning about this
potential performance penalty, even if there is no penalty in that implementation. 

2.1.4. Out-of-bounds access in shaders 

Shader s can access physical resource s either directly
(for example, as a "uniform" GPUBufferBinding ), or via texture unit s,
which are fixed-function hardware blocks that handle texture coordinate conversions.
Validation in the WebGPU API can only guarantee that all the inputs to the shader are provided and
they have the correct usage and types.
The WebGPU API can not guarantee that the data is accessed within bounds
if the texture unit s are not involved. 

In order to prevent the shaders from accessing GPU memory an application doesn’t own,
the WebGPU implementation may enable a special mode (called "robust buffer access") in the driver
that guarantees that the access is limited to buffer bounds. 

Alternatively, an implementation may transform the shader code by inserting manual bounds checks.
When this path is taken, the out-of-bound checks only apply to array indexing. They aren’t needed
for plain field access of shader structures due to the minBindingSize 
validation on the host side. 

If the shader attempts to load data outside of physical resource bounds,
the implementation is allowed to: 

return a value at a different location within the resource bounds 

return a value vector of "(0, 0, 0, X)" with any "X" 

partially discard the draw or dispatch call 

If the shader attempts to write data outside of physical resource bounds,
the implementation is allowed to: 

write the value to a different location within the resource bounds 

discard the write operation 

partially discard the draw or dispatch call 

2.1.5. Invalid data 

When uploading floating-point data from CPU to GPU,
or generating it on the GPU, we may end up with a binary representation that doesn’t correspond
to a valid number, such as infinity or NaN (not-a-number). The GPU behavior in this case is
subject to the accuracy of the GPU hardware implementation of the IEEE-754 standard.
WebGPU guarantees that introducing invalid floating-point numbers would only affect the results
of arithmetic computations and will not have other side effects. 

2.1.6. Driver bugs 

GPU drivers are subject to bugs like any other software. If a bug occurs, an attacker
could possibly exploit the incorrect behavior of the driver to get access to unprivileged data.
In order to reduce the risk, the WebGPU working group will coordinate with GPU vendors
to integrate the WebGPU Conformance Test Suite (CTS) as part of their driver testing process,
like it was done for WebGL.
WebGPU implementations are expected to have workarounds for some of the discovered bugs,
and disable WebGPU on drivers with known bugs that can’t be worked around. 

2.1.7. Timing attacks 

2.1.7.1. Content-timeline timing 

WebGPU does not expose new states to JavaScript (the content timeline ) which are
shared between agents in an agent cluster .
Content timeline states such as [[mapping]] only change during
explicit content timeline tasks, like in plain JavaScript. 

2.1.7.2. Device/queue-timeline timing 

Writable storage buffers and other cross-invocation communication may be usable to construct
high-precision timers on the queue timeline . 

The optional "timestamp-query" feature also provides high precision
timing of GPU operations. To mitigate security and privacy concerns, the timing query
values are aligned to a lower precision: see current queue timestamp . Note in particular: 

The device timeline typically runs in a process that is shared by multiple
origins, so cross-origin isolation (provided by COOP/COEP) does not provide
isolation of device/queue-timeline timers. 

Queue timeline work is issued from the device timeline, and may execute on GPU hardware that
does not provide the isolation expected of CPU processes (such as Meltdown mitigations). 

GPU hardware is not typically susceptible to Spectre-style attacks, but WebGPU may be
implemented in software, and software implementations may run in a shared process, preventing
isolation-based mitigations. 

2.1.8. Row hammer attacks 

Row hammer is a class of attacks that exploit the
leaking of states in DRAM cells. It could be used on GPU .
WebGPU does not have any specific mitigations in place, and relies on platform-level solutions,
such as reduced memory refresh intervals. 

2.1.9. Denial of service 

WebGPU applications have access to GPU memory and compute units. A WebGPU implementation may limit
the available GPU memory to an application, in order to keep other applications responsive.
For GPU processing time, a WebGPU implementation may set up "watchdog" timer that makes sure an
application doesn’t cause GPU unresponsiveness for more than a few seconds.
These measures are similar to those used in WebGL. 

2.1.10. Workload identification 

WebGPU provides access to constrained global resources shared between different programs
(and web pages) running on the same machine. An application can try to indirectly probe
how constrained these global resources are, in order to reason about workloads performed
by other open web pages, based on the patterns of usage of these shared resources.
These issues are generally analogous to issues with Javascript,
such as system memory and CPU execution throughput. WebGPU does not provide any additional
mitigations for this. 

2.1.11. Memory resources 

WebGPU exposes fallible allocations from machine-global memory heaps, such as VRAM.
This allows for probing the size of the system’s remaining available memory
(for a given heap type) by attempting to allocate and watching for allocation failures. 

GPUs internally have one or more (typically only two) heaps of memory
shared by all running applications. When a heap is depleted, WebGPU would fail to create a resource.
This is observable, which may allow a malicious application to guess what heaps
are used by other applications, and how much they allocate from them. 

2.1.12. Computation resources 

If one site uses WebGPU at the same time as another, it may observe the increase
in time it takes to process some work. For example, if a site constantly submits
compute workloads and tracks completion of work on the queue,
it may observe that something else also started using the GPU. 

A GPU has many parts that can be tested independently, such as the arithmetic units,
texture sampling units, atomic units, etc. A malicious application may sense when
some of these units are stressed, and attempt to guess the workload of another
application by analyzing the stress patterns. This is analogous to the realities
of CPU execution of Javascript. 

2.1.13. Abuse of capabilities 

Malicious sites could abuse the capabilities exposed by WebGPU to run
computations that don’t benefit the user or their experience and instead only
benefit the site. Examples would be hidden crypto-mining, password cracking
or rainbow tables computations. 

It is not possible to guard against these types of uses of the API because the
browser is not able to distinguish between valid workloads and abusive
workloads. This is a general problem with all general-purpose computation
capabilities on the Web: JavaScript, WebAssembly or WebGL. WebGPU only makes
some workloads easier to implement, or slightly more efficient to run than
using WebGL. 

To mitigate this form of abuse, browsers can throttle operations on background
tabs, could warn that a tab is using a lot of resource, and restrict which
contexts are allowed to use WebGPU. 

User agents can heuristically issue warnings to users about high power use,
especially due to potentially malicious usage.
If a user agent implements such a warning, it should include WebGPU usage in
its heuristics, in addition to JavaScript, WebAssembly, WebGL, and so on. 

2.2. Privacy Considerations 

The privacy considerations for WebGPU are similar to those of WebGL. GPU APIs are complex and must
expose various aspects of a device’s capabilities out of necessity in order to enable developers to
take advantage of those capabilities effectively. The general mitigation approach involves
normalizing or binning potentially identifying information and enforcing uniform behavior where
possible.

A user agent must not reveal more than 32 distinguishable configurations or buckets. 

2.2.1. Machine-specific features and limits 

WebGPU can expose a lot of detail on the underlying GPU architecture and the device geometry.
This includes available physical adapters, many limits on the GPU and CPU resources
that could be used (such as the maximum texture size), and any optional hardware-specific
capabilities that are available. 

User agents are not obligated to expose the real hardware limits, they are in full control of
how much the machine specifics are exposed. One strategy to reduce fingerprinting is binning
all the target platforms into a few number of bins. In general, the privacy impact of exposing
the hardware limits matches the one of WebGL. 

The default limits are also deliberately high enough
to allow most applications to work without requesting higher limits.
All the usage of the API is validated according to the requested limits,
so the actual hardware capabilities are not exposed to the users by accident. 

2.2.2. Machine-specific artifacts 

There are some machine-specific rasterization/precision artifacts and performance differences
that can be observed roughly in the same way as in WebGL. This applies to rasterization coverage
and patterns, interpolation precision of the varyings between shader stages, compute unit scheduling,
and more aspects of execution. 

Generally, rasterization and precision fingerprints are identical across most or all
of the devices of each vendor. Performance differences are relatively intractable,
but also relatively low-signal (as with JS execution performance). 

Privacy-critical applications and user agents should utilize software implementations to eliminate
such artifacts. 

2.2.3. Machine-specific performance 

Another factor for differentiating users is measuring the performance of specific
operations on the GPU. Even with low precision timing, repeated execution of an operation
can show if the user’s machine is fast at specific workloads.
This is a fairly common vector (present in both WebGL and Javascript),
but it’s also low-signal and relatively intractable to truly normalize. 

WebGPU compute pipelines expose access to GPU unobstructed by the fixed-function hardware.
This poses an additional risk for unique device fingerprinting. User agents can take steps
to dissociate logical GPU invocations with actual compute units to reduce this risk. 

2.2.4. User Agent State 

This specification doesn’t define any additional user-agent state for an origin.
However it is expected that user agents will have compilation caches for the result of expensive
compilation like GPUShaderModule , GPURenderPipeline and GPUComputePipeline .
These caches are important to improve the loading time of WebGPU applications after the first
visit. 

For the specification, these caches are indifferentiable from incredibly fast compilation, but
for applications it would be easy to measure how long createComputePipelineAsync() 
takes to resolve. This can leak information across origins (like "did the user access a site with
this specific shader") so user agents should follow the best practices in
storage partitioning . 

The system’s GPU driver may also have its own cache of compiled shaders and pipelines. User agents
may want to disable these when at all possible, or add per-partition data to shaders in ways that
will make the GPU driver consider them different. 

2.2.5. Driver bugs 

In addition to the concerns outlined in Security Considerations , driver
bugs may introduce differences in behavior that can be observed as a method of differentiating
users. The mitigations mentioned in Security Considerations apply here as well, including
coordinating with GPU vendors and implementing workarounds for known issues in the user agent. 

2.2.6. Adapter Identifiers 

Past experience with WebGL has demonstrated that developers have a legitimate need to be able to
identify the GPU their code is running on in order to create and maintain robust GPU-based content.
For example, to identify adapters with known driver bugs in order to work around them or to avoid
features that perform more poorly than expected on a given class of hardware. 

But exposing adapter identifiers also naturally expands the amount of fingerprinting information
available, so there’s a desire to limit the precision with which we identify the adapter. 

There are several mitigations that can be applied to strike a balance between enabling robust
content and preserving privacy. First is that user agents can reduce the burden on developers by
identifying and working around known driver issues, as they have since browsers began making use of
GPUs. 

When adapter identifiers are exposed by default they should be as broad as possible while still
being useful. Possibly identifying, for example, the adapter’s vendor and general architecture
without identifying the specific adapter in use. Similarly, in some cases identifiers for an adapter
that is considered a reasonable proxy for the actual adapter may be reported. 

In cases where full and detailed information about the adapter is useful (for example: when filing
bug reports) the user can be asked for consent to reveal additional information about their hardware
to the page. 

Finally, the user agent will always have the discretion to not report adapter identifiers at all if
it considers it appropriate, such as in enhanced privacy modes. 

3. Fundamentals 

3.1. Conventions 

3.1.1. Syntactic Shorthands 

In this specification, the following syntactic shorthands are used: 

The . ("dot") syntax, common in programming languages.

The phrasing " Foo.Bar " means "the Bar member of the value (or interface) Foo ."
If Foo is an ordered map and Bar does not exist in Foo , returns undefined . 

The phrasing " Foo.Bar is provided " means
"the Bar member exists in the map value Foo " 

The ?. ("optional chaining") syntax, adopted from JavaScript.

The phrasing " Foo?.Bar " means
"if Foo is null or undefined or Bar does not exist in Foo , undefined ; otherwise, Foo.Bar ". 

For example, where buffer is a GPUBuffer , buffer?.\[[device]].\[[adapter]] means
"if buffer is null or undefined , then undefined ; otherwise,
the \[[adapter]] internal slot of the \[[device]] internal slot of buffer . 

The ?? ("nullish coalescing") syntax, adopted from JavaScript.

The phrasing " x ?? y " means " x , if x is not null or undefined, and y otherwise". 

slot-backed attribute 

A WebIDL attribute which is backed by an internal slot of the same name.
It may or may not be mutable. 

3.1.2. WebGPU Objects

A WebGPU object consists of a WebGPU Interface and an internal object . 

The WebGPU interface defines the public interface and state of the WebGPU object .
It can be used on the content timeline where it was created, where it is a JavaScript-exposed
WebIDL interface. 

Any interface which includes GPUObjectBase is a WebGPU interface . 

The internal object tracks the state of the WebGPU object on the device timeline .
All reads/writes to the mutable state of an internal object occur from steps executing on a
single well-ordered device timeline . 

The following special property types can be defined on WebGPU objects : 

immutable property 

A read-only slot set during initialization of the object. It can be accessed from any timeline. 

Note: Since the slot is immutable, implementations may have a copy on multiple timelines, as needed.
Immutable properties are defined in this way to avoid describing multiple copies in this spec. 

If named [[with brackets]] , it is an internal slot.

If named withoutBrackets , it is a readonly slot-backed attribute of the WebGPU interface . 

content timeline property 

A property which is only accessible from the content timeline 
where the object was created. 

If named [[with brackets]] , it is an internal slot.

If named withoutBrackets , it is a slot-backed attribute of the WebGPU interface . 

device timeline property 

A property which tracks state of the internal object and is only accessible from the
device timeline where the object was created. device timeline properties may be mutable. 

Device timeline properties are named [[with brackets]] , and are internal slots. 

queue timeline property 

A property which tracks state of the internal object and is only accessible from the
queue timeline where the object was created. queue timeline properties may be mutable. 

Queue timeline properties are named [[with brackets]] , and are internal slots. 

interface mixin GPUObjectBase {
attribute USVString label ;
};

To create a new WebGPU object ( GPUObjectBase parent ,
interface T , GPUObjectDescriptorBase descriptor )
(where T extends GPUObjectBase ), run the following content timeline steps:

Let device be parent . [[device]] . 

Let object be a new instance of T . 

Set object . [[device]] to device . 

Set object . label to descriptor . label . 

Return object . 

GPUObjectBase has the following immutable properties : 

[[device]] , of type device , readonly

The device that owns the internal object . 

Operations on the contents of this object assert they are running on the
device timeline , and that the device is valid . 

GPUObjectBase has the following content timeline properties : 

label , of type USVString 

A developer-provided label which is used in an implementation-defined way.
It can be used by the browser, OS, or other tools to help
identify the underlying internal object to the developer.
Examples include displaying the label in GPUError messages, console warnings,
browser developer tools, and platform debugging utilities. 

NOTE: 

Implementations should use labels to enhance error messages by using them to
identify WebGPU objects.

However, this need not be the only way of identifying objects:
implementations should also use other available information,
especially when no label is available. For example: 

The label of the parent GPUTexture when printing a GPUTextureView . 

The label of the parent GPUCommandEncoder when printing a
GPURenderPassEncoder or GPUComputePassEncoder . 

The label of the source GPUCommandEncoder when printing a GPUCommandBuffer . 

The label of the source GPURenderBundleEncoder when printing a GPURenderBundle . 

NOTE: 

The label is a property of the GPUObjectBase .
Two GPUObjectBase "wrapper" objects have completely separate label states,
even if they refer to the same underlying object
(for example returned by getBindGroupLayout() ).
The label property will not change except by being set from JavaScript.

This means one underlying object could be associated with multiple labels.
This specification does not define how the label is propagated to the device timeline .
How labels are used is completely implementation-defined : error messages could
show the most recently set label, all known labels, or no labels at all. 

It is defined as a USVString because some user agents may
supply it to the debug facilities of the underlying native APIs. 

GPUObjectBase has the following device timeline properties : 

[[valid]] , of type boolean , initially true .

If true , indicates that the internal object is valid to use. 

NOTE: 

Ideally WebGPU interfaces should not prevent their parent objects, such as the
[[device]] that owns them, from being garbage collected. This cannot be
guaranteed, however, as holding a strong reference to a parent object may be required in some
implementations.

As a result, developers should assume that a WebGPU interface may remain live
until all child objects of that interface have also been garbage collected, causing some
resources to remain allocated longer than anticipated. 

Calling the destroy method on a WebGPU interface (such as
GPUDevice . destroy() or GPUBuffer . destroy() ) should be
favored over relying on garbage collection if predictable release of allocated resources is
needed. 

3.1.3. Object Descriptors 

An object descriptor holds the information needed to create an object,
which is typically done via one of the create* methods of GPUDevice . 

dictionary GPUObjectDescriptorBase {
USVString label = "";
};

GPUObjectDescriptorBase has the following members: 

label , of type USVString , defaulting to "" 

The initial value of GPUObjectBase.label . 

3.2. Asynchrony 

3.2.1. Invalid Internal Objects & Contagious Invalidity 

Object creation operations in WebGPU don’t return promises, but nonetheless are internally
asynchronous. Returned objects refer to internal objects which are manipulated on a
device timeline . Rather than fail with exceptions or rejections, most errors that occur on a
device timeline are communicated through GPUError s generated on the associated device . 

Internal objects are either valid or invalid .
An invalid object will never become valid at a later time,
but some valid objects may be invalidated . 

Objects are invalid from creation if it wasn’t possible to create them.
This can happen, for example, if the object descriptor doesn’t describe a valid
object, or if there is not enough memory to allocate a resource.
It can also happen if an object is created with or from another invalid object
(for example calling createView() on an invalid GPUTexture )
(for example the GPUTexture of a createView() call):
this case is referred to as contagious invalidity . 

Internal objects of most types cannot become invalid after they are created, but still
may become unusable, e.g. if the owning device is lost or
destroyed , or the object has a special internal state,
like buffer state " destroyed ". 

Internal objects of some types can become invalid after they are created; specifically,
devices , adapters , GPUCommandBuffer s, and command/pass/bundle encoders. 

A given GPUObjectBase object is valid if
object . [[valid]] is true .

A given GPUObjectBase object is invalid if
object . [[valid]] is false .

A given GPUObjectBase object is valid to use with 
a targetObject if the all of the requirements in the following device timeline steps are met:

object . [[valid]] must be true . 

object . [[device]] . [[valid]] must be true . 

object . [[device]] must equal targetObject . [[device]] . 

To invalidate a GPUObjectBase 
object , run the following device timeline steps:

object . [[valid]] to false . 

3.2.2. Promise Ordering 

Several operations in WebGPU return promises. 

GPU . requestAdapter() 

GPUAdapter . requestDevice() 

GPUDevice . createComputePipelineAsync() 

GPUDevice . createRenderPipelineAsync() 

GPUShaderModule . getCompilationInfo() 

GPUQueue . onSubmittedWorkDone() 

GPUBuffer . mapAsync() 

GPUDevice . lost 

GPUDevice . popErrorScope() 

WebGPU does not make any guarantees about the order in which these promises settle
(resolve or reject), except for the following: 

For some GPUQueue q ,
if p1 = q . onSubmittedWorkDone() is called before
p2 = q . onSubmittedWorkDone() ,
then p1 must settle before p2 .

For some GPUQueue q and GPUBuffer b on the same GPUDevice ,
if p1 = b . mapAsync() is called before
p2 = q . onSubmittedWorkDone() ,
then p1 must settle before p2 .

Applications must not rely on any other promise settlement ordering. 

3.3. Coordinate Systems 

Rendering operations use the following coordinate systems: 

Normalized device coordinates (or NDC) have three dimensions, where: 

-1.0 ≤ x ≤ 1.0 

-1.0 ≤ y ≤ 1.0 

0.0 ≤ z ≤ 1.0 

The bottom-left corner is at (-1.0, -1.0, z). 

Normalized device coordinates. 

Note: Whether z = 0 or z = 1 is treated as the near plane is application specific. The above diagram presents
z = 0 as the near plane but the observed behavior is determined by a combination of the projection matrices
used by shaders, the depthClearValue , and the
depthCompare function. 

Clip space coordinates have four dimensions: (x, y, z, w) 

Clip space coordinates are used for the the clip position of a vertex (i.e. the position output of a vertex shader),
and for the clip volume . 

Normalized device coordinates and clip space coordinates are related as follows:
If point p = (p.x, p.y, p.z, p.w) is in the clip volume , then its normalized device coordinates are ( p.x ÷ p.w , p.y ÷ p.w , p.z ÷ p.w ). 

Framebuffer coordinates address the pixels in the framebuffer 

They have two dimensions. 

Each pixel extends 1 unit in x and y dimensions. 

The top-left corner is at (0.0, 0.0). 

x increases to the right. 

y increases down. 

See § 17 Render Passes and § 23.2.5 Rasterization . 

Framebuffer coordinates. 

Viewport coordinates combine framebuffer coordinates in x and y dimensions,
with depth in z. 

Normally 0.0 ≤ z ≤ 1.0, but this can be modified by setting [[viewport]] . minDepth and maxDepth via
setViewport() 

Fragment coordinates match viewport coordinates . 

Texture coordinates , sometimes called "UV coordinates" in 2D, are used to sample
textures and have a number of components matching the texture dimension . 

0 ≤ u ≤ 1.0 

0 ≤ v ≤ 1.0 

0 ≤ w ≤ 1.0 

(0.0, 0.0, 0.0) is in the first texel in texture memory address order. 

(1.0, 1.0, 1.0) is in the last texel texture memory address order. 

2D Texture coordinates. 

Window coordinates , or present coordinates ,
match framebuffer coordinates , and are used when interacting with
an external display or conceptually similar interface. 

Note: WebGPU’s coordinate systems match DirectX’s coordinate systems in a graphics pipeline. 

3.4. Programming Model 

3.4.1. Timelines 

WebGPU’s behavior is described in terms of "timelines".
Each operation (defined as algorithms) occurs on a timeline.
Timelines clearly define both the order of operations, and which state is
available to which operations. 

Note: 
This "timeline" model describes the constraints of the multi-process models of
browser engines (typically with a "content process" and "GPU process"), as well
as the GPU itself as a separate execution unit in many implementations.
Implementing WebGPU does not require timelines to execute in parallel, so does
not require multiple processes, or even multiple threads.
(It does require concurrency for cases like get a copy of the image contents of a context 
which synchronously blocks on another timeline to complete.) 

Content timeline 

Associated with the execution of the Web script.
It includes calling all methods described by this specification. 

To issue steps to the content timeline from an operation on GPUDevice device ,
queue a global task for GPUDevice device with those steps. 

Device timeline 

Associated with the GPU device operations
that are issued by the user agent.
It includes creation of adapters, devices, and GPU resources
and state objects, which are typically synchronous operations from the point
of view of the user agent part that controls the GPU,
but can live in a separate OS process. 

Queue timeline 

Associated with the execution of operations
on the compute units of the GPU. It includes actual draw, copy,
and compute jobs that run on the GPU. 

Timeline-agnostic 

Associated with any of the above timelines 

Steps may be issued to any timeline if they only operate on immutable properties or
arguments passed from the calling steps. 

The following show the styling of steps and values associated with each timeline.
This styling is non-normative; the specification text always describes the association.

Immutable value example term definition

Can be used on any timeline. 

Content-timeline example term definition

Can only be used on the content timeline . 

Device-timeline example term definition

Can only be used on the device timeline . 

Queue-timeline example term definition

Can only be used on the queue timeline . 

Steps which are timeline-agnostic look like this.

Immutable value example term usage. 

Steps executed on the content timeline look like this.

Immutable value example term usage.
Content-timeline example term usage. 

Steps executed on the device timeline look like this.

Immutable value example term usage.
Device-timeline example term usage. 

Steps executed on the queue timeline look like this.

Immutable value example term usage.
Queue-timeline example term usage. 

In this specification, asynchronous operations are used when the return value
depends on work that happens on any timeline other than the Content timeline .
They are represented by promises and events in API. 

GPUComputePassEncoder.dispatchWorkgroups() :

User encodes a dispatchWorkgroups command by calling a method of the
GPUComputePassEncoder which happens on the Content timeline . 

User issues GPUQueue.submit() that hands over
the GPUCommandBuffer to the user agent, which processes it
on the Device timeline by calling the OS driver to do a low-level submission. 

The submit gets dispatched by the GPU invocation scheduler onto the
actual compute units for execution, which happens on the Queue timeline . 

GPUDevice.createBuffer() :

User fills out a GPUBufferDescriptor and creates a GPUBuffer with it,
which happens on the Content timeline . 

User agent creates a low-level buffer on the Device timeline . 

GPUBuffer.mapAsync() :

User requests to map a GPUBuffer on the Content timeline and
gets a promise in return. 

User agent checks if the buffer is currently used by the GPU
and makes a reminder to itself to check back when this usage is over. 

After the GPU operating on Queue timeline is done using the buffer,
the user agent maps it to memory and resolves the promise. 

3.4.2. Memory Model 

This section is non-normative. 

Once a GPUDevice has been obtained during an application initialization routine,
we can describe the WebGPU platform as consisting of the following layers: 

User agent implementing the specification. 

Operating system with low-level native API drivers for this device. 

Actual CPU and GPU hardware. 

Each layer of the WebGPU platform may have different memory types
that the user agent needs to consider when implementing the specification: 

The script-owned memory, such as an ArrayBuffer created by the script,
is generally not accessible by a GPU driver. 

A user agent may have different processes responsible for running
the content and communication to the GPU driver.
In this case, it uses inter-process shared memory to transfer data. 

Dedicated GPUs have their own memory with high bandwidth,
while integrated GPUs typically share memory with the system. 

Most physical resources are allocated in the memory of type
that is efficient for computation or rendering by the GPU.
When the user needs to provide new data to the GPU,
the data may first need to cross the process boundary in order to reach
the user agent part that communicates with the GPU driver.
Then it may need to be made visible to the driver,
which sometimes requires a copy into driver-allocated staging memory.
Finally, it may need to be transferred to the dedicated GPU memory,
potentially changing the internal layout into one
that is most efficient for GPUs to operate on. 

All of these transitions are done by the WebGPU implementation of the user agent. 

Note: This example describes the worst case, while in practice
the implementation might not need to cross the process boundary,
or may be able to expose the driver-managed memory directly to
the user behind an ArrayBuffer , thus avoiding any data copies. 

3.4.3. Resource Usages 

A physical resource can be used with an internal usage by a GPU command : 

input 

Buffer with input data for draw or dispatch calls. Preserves the contents.
Allowed by buffer INDEX , buffer VERTEX , or buffer INDIRECT . 

constant 

Resource bindings that are constant from the shader point of view. Preserves the contents.
Allowed by buffer UNIFORM or texture TEXTURE_BINDING . 

storage 

Read/write storage resource binding.
Allowed by buffer STORAGE or texture STORAGE_BINDING . 

storage-read 

Read-only storage resource bindings. Preserves the contents.
Allowed by buffer STORAGE or texture STORAGE_BINDING . 

attachment 

Texture used as a read/write output attachment or
write-only resolve target in a render pass.
Allowed by texture RENDER_ATTACHMENT . 

attachment-read 

Texture used as a read-only attachment in a render pass. Preserves the contents.
Allowed by texture RENDER_ATTACHMENT . 

We define subresource to be either a whole buffer, or a texture subresource . 

Some internal usages are compatible with others. A subresource can be in a state
that combines multiple usages together. We consider a list U to be
a compatible usage list if (and only if) it satisfies any of the following rules:

Each usage in U is input , constant , storage-read , or attachment-read . 

Each usage in U is storage . 

Multiple such usages are allowed even though they are writable.
This is the usage scope storage exception . 

Each usage in U is attachment . 

Multiple such usages are allowed even though they are writable.
This is the usage scope attachment exception . 

Enforcing that the usages are only combined into a compatible usage list 
allows the API to limit when data races can occur in working with memory.
That property makes applications written against
WebGPU more likely to run without modification on different platforms. 

EXAMPLE: 

Binding the same buffer for storage as well as for
input within the same GPURenderPassEncoder 
results in a non- compatible usage list for that buffer.

EXAMPLE: 

These rules allow for read-only depth-stencil : a single depth/stencil
texture can be used as two different read-only usages in a render pass simultaneously:

attachment-read 

As a depth/stencil attachment with all aspects marked read-only
(using depthReadOnly and/or
stencilReadOnly as necessary). 

constant 

As a texture binding to a draw call. 

EXAMPLE: 

The usage scope storage exception allows two cases that would
not be allowed otherwise:

A buffer or texture may be bound as storage to two
different draw calls in a render pass. 

Disjoint ranges of a single buffer may be bound to two different binding
points as storage . 

Overlapping ranges must not be bound to a single dispatch/draw call;
this is checked by " Encoder bind groups alias a writable resource ". 

EXAMPLE: 

The usage scope attachment exception allows a texture subresource
to be used as attachment more than once.
This is necessary to allow disjoint slices of 3D textures
to be bound as different attachments to a single render pass.

One slice must not be bound twice for two different attachments;
this is checked by beginRenderPass() . 

3.4.4. Synchronization 

A usage scope is a map from subresource to list < internal usage >>.
Each usage scope covers a range of operations which may execute in a concurrent
fashion with each other, and therefore may only use subresources in consistent
compatible usage lists within the scope. 

A usage scope scope passes usage scope validation if,
for each [ subresource , usageList ] in scope ,
usageList is a compatible usage list .

To add 
a subresource subresource to usage scope usageScope with usage
( internal usage or set of internal usages ) usage :

If usageScope [ subresource ] does not exist , set it to [] . 

Append usage to usageScope [ subresource ]. 

To merge 
usage scope A into usage scope B :

For each [ subresource , usage ] in A : 

Add subresource to B with usage usage . 

Usage scopes are constructed and validated during encoding: 

in dispatchWorkgroups() 

in dispatchWorkgroupsIndirect() 

at GPURenderPassEncoder.end() 

at GPURenderBundleEncoder.finish() 

The usage scopes are as follows: 

In a compute pass, each dispatch command ( dispatchWorkgroups() or
dispatchWorkgroupsIndirect() ) is one usage scope. 

A subresource is used in the usage scope if it is
potentially accessible by the dispatched invocations, including: 

All subresources referenced by bind groups in slots used by the current
GPUComputePipeline ’s [[layout]] 

Buffers used directly by dispatch calls (such as indirect buffers) 

Note: 
State-setting compute pass commands, like
setBindGroup() ,
do not contribute their bound resources directly to a usage scope: they only change the
state that is checked in dispatch commands. 

One render pass is one usage scope. 

A subresource is used in the usage scope if it’s referenced by any command,
including state-setting commands (unlike in compute passes), including: 

Buffers set by setVertexBuffer() 

Buffers set by setIndexBuffer() 

All subresources referenced by bind groups set by
setBindGroup() 

Buffers used directly by draw calls (such as indirect buffers) 

Note: Copy commands are standalone operations and don’t use usage scopes for validation.
They implement their own validation to prevent self-races. 

EXAMPLE: 

The following example resource usages are included in usage scopes :

In a render pass, subresources used in any
setBindGroup() 
call, regardless of whether the currently bound pipeline’s
shader or layout actually depends on these bindings,
or the bind group is shadowed by another 'set' call. 

A buffer used in any setVertexBuffer() 
call, regardless of whether any draw call depends on this buffer,
or whether this buffer is shadowed by another 'set' call. 

A buffer used in any setIndexBuffer() 
call, regardless of whether any draw call depends on this buffer,
or whether this buffer is shadowed by another 'set' call. 

A texture subresource used as a color attachment, resolve attachment, or
depth/stencil attachment in GPURenderPassDescriptor by
beginRenderPass() ,
regardless of whether the shader actually depends on these attachments. 

Resources used in bind group entries with visibility 0, or visible only
to the compute stage but used in a render pass (or vice versa). 

3.5. Core Internal Objects 

3.5.1. Adapters 

An adapter identifies an implementation of WebGPU on the system:
both an instance of compute/rendering functionality on the
platform underlying a browser, and an instance of a browser’s implementation of
WebGPU on top of that functionality. 

Adapters are exposed via GPUAdapter . 

Adapters do not uniquely represent underlying implementations:
calling requestAdapter() multiple times returns a different adapter 
object each time. 

Each adapter object can only be used to create one device :
upon a successful requestDevice() call, the adapter’s [[state]] 
changes to "consumed" .
Additionally, adapter objects may expire at any time. 

Note: 
This ensures applications use the latest system state for adapter selection when creating a device.
It also encourages robustness to more scenarios by making them look similar: first initialization,
reinitialization due to an unplugged adapter, reinitialization due to a test
GPUDevice.destroy() call, etc. 

An adapter may be considered a fallback adapter if it has significant performance
caveats in exchange for some combination of wider compatibility, more predictable behavior, or
improved privacy. It is not required that a fallback adapter is available on every system. 

adapter has the following immutable properties : 

[[features]] , of type ordered set < GPUFeatureName >, readonly

The features which can be used to create devices on this adapter. 

[[limits]] , of type supported limits , readonly

The best limits which can be used to create devices on this adapter. 

Each adapter limit must be the same or better than its default value
in supported limits . 

[[fallback]] , of type boolean , readonly

If set to true indicates that the adapter is a fallback adapter . 

[[xrCompatible]] , of type boolean

If set to true indicates that the adapter was requested with compatibility with
WebXR sessions . 

[[default feature level]] , of type feature level string , readonly

Indicates the default feature level of devices created from this adapter. 

adapter has the following device timeline properties : 

[[state]] , initially "valid" 

"valid" 

The adapter can be used to create a device. 

"consumed" 

The adapter has already been used to create a device, and cannot be used again. 

"expired" 

The adapter has expired for some other reason. 

To expire a GPUAdapter adapter , run the
following device timeline steps:

Set adapter . [[adapter]] . [[state]] to
"expired" . 

3.5.2. Devices 

A device is the logical instantiation of an adapter ,
through which internal objects are created. 

Devices are exposed via GPUDevice . 

A device is the exclusive owner of all internal objects created from it:
when the device becomes invalid 
(is lost or destroyed ),
it and all objects created on it (directly, e.g.
createTexture() , or indirectly, e.g. createView() ) become
implicitly unusable . 

device has the following immutable properties : 

[[adapter]] , of type adapter , readonly

The adapter from which this device was created. 

[[features]] , of type ordered set < GPUFeatureName >, readonly

The features which can be used on this device, as computed at creation .
No additional features can be used, even if the underlying adapter can support them. 

[[limits]] , of type supported limits , readonly

The limits which can be used on this device, as computed at creation .
No better limits can be used, even if the underlying adapter can support them. 

device has the following content timeline properties : 

[[content device]] , of type GPUDevice , readonly

The Content timeline GPUDevice interface which this device is associated with. 

To create a new device from adapter adapter 
with GPUDeviceDescriptor descriptor , run the following device timeline steps:

Let features be the set of values in
descriptor . requiredFeatures . 

If features contains "texture-formats-tier2" : 

Append "texture-formats-tier1" to features . 

If features contains "texture-formats-tier1" : 

Append "rg11b10ufloat-renderable" to features . 

Append any default GPUFeatureName s to features 
as defined by the adapter . [[default feature level]] . 

Let limits be a new supported limits object with the default limits
as defined by the adapter . [[default feature level]] . 

For each ( key , value ) pair in descriptor . requiredLimits : 

If value is not undefined and value is better than limits [ key ]: 

Set limits [ key ] to value . 

Set limits . maxStorageBuffersPerShaderStage to max( limits . maxStorageBuffersPerShaderStage , limits . maxStorageBuffersInVertexStage , limits . maxStorageBuffersInFragmentStage ). 

Set limits . maxStorageTexturesPerShaderStage to max( limits . maxStorageTexturesPerShaderStage , limits . maxStorageTexturesInVertexStage , limits . maxStorageTexturesInFragmentStage ). 

If features contains "core-features-and-limits" : 

Set limits . maxStorageBuffersInVertexStage and limits . maxStorageBuffersInFragmentStage to limits . maxStorageBuffersPerShaderStage . 

Set limits . maxStorageTexturesInVertexStage and limits . maxStorageTexturesInFragmentStage to limits . maxStorageTexturesPerShaderStage . 

Let device be a device object. 

Set device . [[adapter]] to adapter . 

Set device . [[features]] to features . 

Set device . [[limits]] to limits . 

Return device . 

Any time the user agent needs to revoke access to a device, it calls
lose the device ( device , "unknown" ) on the device’s device timeline ,
potentially ahead of other operations currently queued on that timeline. 

If an operation fails with side effects that would observably change the state
of objects on the device or potentially corrupt internal implementation/driver state,
the device should be lost to prevent these changes from being observable. 

Note: 
For all device losses not initiated by the application (via destroy() ),
user agents should consider issuing developer-visible warnings unconditionally ,
even if the lost promise is handled.
These scenarios should be rare, and the signal is vital to developers because most of the WebGPU
API tries to behave like nothing is wrong to avoid interrupting the runtime flow of the application:
no validation errors are raised, most promises resolve normally, etc. 

To lose the device ( device , reason ) run the following device timeline steps:

Invalidate device . 

Issue the following steps on the content timeline of device . [[content device]] : 

Resolve device . lost with a new GPUDeviceLostInfo with
reason set to reason and
message set to an implementation-defined value. 

Note: message should not disclose unnecessary user/system
information and should never be parsed by applications. 

Complete any outstanding steps that are waiting until device becomes lost . 

Note: No errors are generated from a device which is lost.
See § 22 Errors & Debugging . 

To listen for timeline event 
event on device device , handled by steps on timeline timeline :

If or when the device timeline has been informed of the completion of event , or 

If device is lost already, or when it becomes lost : 

Then issue steps on timeline . 

3.6. Optional Capabilities 

WebGPU adapters and devices have capabilities , which
describe WebGPU functionality that differs between different implementations,
typically due to hardware or system software constraints.
A capability is either a feature or a limit . 

A user agent must not reveal more than 32 distinguishable configurations or buckets. 

The capabilities of an adapter must conform to § 4.2.1 Adapter Capability Guarantees . 

Only supported capabilities may be requested in requestDevice() ;
requesting unsupported capabilities results in failure. 

The capabilities of a device are determined in " a new device " by starting with the adapter’s
defaults (no features and the default supported limits )
and adding capabilities as requested in requestDevice() .
These capabilities are enforced regardless of the capabilities of the adapter . 

For privacy considerations, see § 2.2.1 Machine-specific features and limits .

3.6.1. Features 

A feature is a set of optional WebGPU functionality that is not supported
on all implementations, typically due to hardware or system software constraints. 

All features are optional, but adapters make some guarantees about their availability
(see § 4.2.1 Adapter Capability Guarantees ). 

A device supports the exact set of features determined at creation (see § 3.6 Optional Capabilities ).
API calls perform validation according to these features (not the adapter ’s features): 

Using existing API surfaces in a new way typically results in a validation error . 

There are several types of optional API surface : 

Using a new method or enum value always throws a TypeError . 

Using a new dictionary member with a (correctly-typed) non-default value typically 
results in a validation error . 

Using a new WGSL enable directive always results in a createShaderModule() 
validation error . 

A GPUFeatureName feature is enabled for 
a GPUObjectBase object if and only if
object . [[device]] . [[features]] contains feature .

See the Feature Index for a description of the functionality each feature enables. 

Note: 
Even where supported, enabling features is not necessarily desirable, as doing so may have a performance impact.
Because of this, and to improve portability across devices and implementations,
applications should generally only request features that they may actually require. 

3.6.2. Limits 

Each limit is a numeric limit on the usage of WebGPU on a device. 

Note: 
Even where supported, setting "better" limits is not necessarily desirable, as doing so may have a performance impact.
Because of this, and to improve portability across devices and implementations, applications should
generally only request limits better than the defaults if they may actually require them. 

Each limit has a default value and a compatibility mode default . 

Adapters are always guaranteed to support the defaults or better 
(see § 4.2.1 Adapter Capability Guarantees ). 

A device supports the exact set of limits determined at creation (see § 3.6 Optional Capabilities ).
API calls perform validation according to these limits (not the adapter ’s limits),
no better or worse. 

For any given limit, some values are better than others.
A better limit value always relaxes validation, enabling strictly
more programs to be valid. For each limit class , "better" is defined. 

Different limits have different limit classes : 

maximum 

The limit enforces a maximum on some value passed into the API. 

Higher values are better . 

May only be set to values ≥ the default .
Lower values are clamped to the default . 

alignment 

The limit enforces a minimum alignment on some value passed into the API; that is,
the value must be a multiple of the limit. 

Lower values are better . 

May only be set to powers of 2 which are ≤ the default .
Values which are not powers of 2 are invalid.
Higher powers of 2 are clamped to the default . 

A supported limits object has a value for every limit defined by WebGPU: 

Limit name 
Type 
Limit class 
Default 
Compatibility Mode Default 

maxTextureDimension1D 

GPUSize32 
maximum 
8192 
4096

The maximum allowed value for the size . width 
of a texture created with dimension "1d" .

maxTextureDimension2D 

GPUSize32 
maximum 
8192 
4096

The maximum allowed value for the size . width and size . height 
of a texture created with dimension "2d" .

maxTextureDimension3D 

GPUSize32 
maximum 
2048

The maximum allowed value for the size . width , size . height and size . depthOrArrayLayers 
of a texture created with dimension "3d" .

maxTextureArrayLayers 

GPUSize32 
maximum 
256

The maximum allowed value for the size . depthOrArrayLayers 
of a texture created with dimension "2d" .

maxBindGroups 

GPUSize32 
maximum 
4

The maximum number of GPUBindGroupLayouts 
allowed in bindGroupLayouts 
when creating a GPUPipelineLayout .

maxBindGroupsPlusVertexBuffers 

GPUSize32 
maximum 
24

The maximum number of bind group and vertex buffer slots used simultaneously,
counting any empty slots below the highest index.
Validated in createRenderPipeline() and in draw calls .

maxBindingsPerBindGroup 

GPUSize32 
maximum 
1000

The number of binding indices available when creating a GPUBindGroupLayout .

Note: This limit is normative, but arbitrary.
With the default binding slot limits , it is impossible
to use 1000 bindings in one bind group, but this allows
GPUBindGroupLayoutEntry . binding values up to 999.
This limit allows implementations to treat binding space as an array,
within reasonable memory space, rather than a sparse map structure. 

maxDynamicUniformBuffersPerPipelineLayout 

GPUSize32 
maximum 
8

The maximum number of GPUBindGroupLayoutEntry entries across a GPUPipelineLayout 
which are uniform buffers with dynamic offsets.
See Exceeds the binding slot limits .

maxDynamicStorageBuffersPerPipelineLayout 

GPUSize32 
maximum 
4

The maximum number of GPUBindGroupLayoutEntry entries across a GPUPipelineLayout 
which are storage buffers with dynamic offsets.
See Exceeds the binding slot limits .

maxSampledTexturesPerShaderStage 

GPUSize32 
maximum 
16

For each possible GPUShaderStage stage ,
the maximum number of GPUBindGroupLayoutEntry entries across a GPUPipelineLayout 
which are sampled textures.
See Exceeds the binding slot limits .

maxSamplersPerShaderStage 

GPUSize32 
maximum 
16

For each possible GPUShaderStage stage ,
the maximum number of GPUBindGroupLayoutEntry entries across a GPUPipelineLayout 
which are samplers.
See Exceeds the binding slot limits .

maxStorageBuffersPerShaderStage 

GPUSize32 
maximum 
8

For each possible GPUShaderStage stage ,
the maximum number of GPUBindGroupLayoutEntry entries across a GPUPipelineLayout 
which are storage buffers.
See Exceeds the binding slot limits .

Note: This limit applies to all stages. At device initialization , it is normalized with maxStorageBuffersInVertexStage and maxStorageBuffersInFragmentStage so that in the validation algorithm, each stage can be checked against just one of the three limits. 

maxStorageBuffersInVertexStage 

GPUSize32 
maximum 
8 
0

For the vertex stage, the maximum number of GPUBindGroupLayoutEntry entries across a GPUPipelineLayout 
which are storage buffers.
See Exceeds the binding slot limits .

maxStorageBuffersInFragmentStage 

GPUSize32 
maximum 
8 
4

For the fragment stage, the maximum number of GPUBindGroupLayoutEntry entries across a GPUPipelineLayout 
which are storage buffers.
See Exceeds the binding slot limits .

maxStorageTexturesPerShaderStage 

GPUSize32 
maximum 
4

For each possible GPUShaderStage stage ,
the maximum number of GPUBindGroupLayoutEntry entries across a GPUPipelineLayout 
which are storage textures.
See Exceeds the binding slot limits .

Note: This limit applies to all stages. At device initialization , it is normalized with maxStorageTexturesInVertexStage and maxStorageTexturesInFragmentStage so that in the validation algorithm, each stage can be checked against just one of the three limits. 

maxStorageTexturesInVertexStage 

GPUSize32 
maximum 
4 
0

For the vertex stage, the maximum number of GPUBindGroupLayoutEntry entries across a GPUPipelineLayout 
which are storage textures.
See Exceeds the binding slot limits .

maxStorageTexturesInFragmentStage 

GPUSize32 
maximum 
4

For the fragment stage, the maximum number of GPUBindGroupLayoutEntry entries across a GPUPipelineLayout 
which are storage textures.
See Exceeds the binding slot limits .

maxUniformBuffersPerShaderStage 

GPUSize32 
maximum 
12

For each possible GPUShaderStage stage ,
the maximum number of GPUBindGroupLayoutEntry entries across a GPUPipelineLayout 
which are uniform buffers.
See Exceeds the binding slot limits .

maxUniformBufferBindingSize 

GPUSize64 
maximum 
65536 bytes 
16384 bytes

The maximum GPUBufferBinding . size for bindings with a
GPUBindGroupLayoutEntry entry for which
entry . buffer ?. type 
is "uniform" .

maxStorageBufferBindingSize 

GPUSize64 
maximum 
134217728 bytes (128 MiB)

The maximum GPUBufferBinding . size for bindings with a
GPUBindGroupLayoutEntry entry for which
entry . buffer ?. type 
is "storage" 
or "read-only-storage" .

minUniformBufferOffsetAlignment 

GPUSize32 
alignment 
256 bytes

The required alignment for GPUBufferBinding . offset and
the dynamic offsets provided in setBindGroup() ,
for bindings with a GPUBindGroupLayoutEntry entry for which
entry . buffer ?. type 
is "uniform" .

minStorageBufferOffsetAlignment 

GPUSize32 
alignment 
256 bytes

The required alignment for GPUBufferBinding . offset and
the dynamic offsets provided in setBindGroup() ,
for bindings with a GPUBindGroupLayoutEntry entry for which
entry . buffer ?. type 
is "storage" 
or "read-only-storage" .

maxVertexBuffers 

GPUSize32 
maximum 
8

The maximum number of buffers 
when creating a GPURenderPipeline .

maxBufferSize 

GPUSize64 
maximum 
268435456 bytes (256 MiB)

The maximum size of size 
when creating a GPUBuffer .

maxVertexAttributes 

GPUSize32 
maximum 
16

The maximum number of attributes 
in total across buffers 
when creating a GPURenderPipeline .

maxVertexBufferArrayStride 

GPUSize32 
maximum 
2048 bytes

The maximum allowed arrayStride 
when creating a GPURenderPipeline .

maxInterStageShaderVariables 

GPUSize32 
maximum 
16 
15

The maximum allowed number of input or output variables for inter-stage
communication (like vertex outputs or fragment inputs).

maxColorAttachments 

GPUSize32 
maximum 
8 
4

The maximum allowed number of color attachments in
GPURenderPipelineDescriptor . fragment . targets ,
GPURenderPassDescriptor . colorAttachments ,
and GPURenderPassLayout . colorFormats .

maxColorAttachmentBytesPerSample 

GPUSize32 
maximum 
32

The maximum number of bytes necessary to hold one sample (pixel or subpixel)
of render pipeline output data, across all color attachments.

maxComputeWorkgroupStorageSize 

GPUSize32 
maximum 
16384 bytes

The maximum number of bytes of workgroup storage used for a compute stage
GPUShaderModule entry-point.

maxComputeInvocationsPerWorkgroup 

GPUSize32 
maximum 
256 
128

The maximum value of the product of the workgroup_size dimensions for a
compute stage GPUShaderModule entry-point.

maxComputeWorkgroupSizeX 

GPUSize32 
maximum 
256 
128

The maximum value of the workgroup_size X dimension for a
compute stage GPUShaderModule entry-point.

maxComputeWorkgroupSizeY 

GPUSize32 
maximum 
256 
128

The maximum value of the workgroup_size Y dimensions for a
compute stage GPUShaderModule entry-point.

maxComputeWorkgroupSizeZ 

GPUSize32 
maximum 
64

The maximum value of the workgroup_size Z dimensions for a
compute stage GPUShaderModule entry-point.

maxComputeWorkgroupsPerDimension 

GPUSize32 
maximum 
65535

The maximum value for the arguments of
dispatchWorkgroups(workgroupCountX, workgroupCountY, workgroupCountZ) .

3.6.2.1. GPUSupportedLimits 

GPUSupportedLimits exposes an adapter or device’s supported limits .
See GPUAdapter.limits and GPUDevice.limits . 

[ Exposed =( Window , Worker ), SecureContext ]
interface GPUSupportedLimits {
readonly attribute unsigned long maxTextureDimension1D ;
readonly attribute unsigned long maxTextureDimension2D ;
readonly attribute unsigned long maxTextureDimension3D ;
readonly attribute unsigned long maxTextureArrayLayers ;
readonly attribute unsigned long maxBindGroups ;
readonly attribute unsigned long maxBindGroupsPlusVertexBuffers ;
readonly attribute unsigned long maxBindingsPerBindGroup ;
readonly attribute unsigned long maxDynamicUniformBuffersPerPipelineLayout ;
readonly attribute unsigned long maxDynamicStorageBuffersPerPipelineLayout ;
readonly attribute unsigned long maxSampledTexturesPerShaderStage ;
readonly attribute unsigned long maxSamplersPerShaderStage ;
readonly attribute unsigned long maxStorageBuffersPerShaderStage ;
readonly attribute unsigned long maxStorageBuffersInVertexStage ;
readonly attribute unsigned long maxStorageBuffersInFragmentStage ;
readonly attribute unsigned long maxStorageTexturesPerShaderStage ;
readonly attribute unsigned long maxStorageTexturesInVertexStage ;
readonly attribute unsigned long maxStorageTexturesInFragmentStage ;
readonly attribute unsigned long maxUniformBuffersPerShaderStage ;
readonly attribute unsigned long long maxUniformBufferBindingSize ;
readonly attribute unsigned long long maxStorageBufferBindingSize ;
readonly attribute unsigned long minUniformBufferOffsetAlignment ;
readonly attribute unsigned long minStorageBufferOffsetAlignment ;
readonly attribute unsigned long maxVertexBuffers ;
readonly attribute unsigned long long maxBufferSize ;
readonly attribute unsigned long maxVertexAttributes ;
readonly attribute unsigned long maxVertexBufferArrayStride ;
readonly attribute unsigned long maxInterStageShaderVariables ;
readonly attribute unsigned long maxColorAttachments ;
readonly attribute unsigned long maxColorAttachmentBytesPerSample ;
readonly attribute unsigned long maxComputeWorkgroupStorageSize ;
readonly attribute unsigned long maxComputeInvocationsPerWorkgroup ;
readonly attribute unsigned long maxComputeWorkgroupSizeX ;
readonly attribute unsigned long maxComputeWorkgroupSizeY ;
readonly attribute unsigned long maxComputeWorkgroupSizeZ ;
readonly attribute unsigned long maxComputeWorkgroupsPerDimension ;
};

3.6.2.2. GPUSupportedFeatures 

GPUSupportedFeatures is a setlike interface. Its set entries are
the GPUFeatureName values of the features supported by an adapter or
device. It must only contain strings from the GPUFeatureName enum. 

[ Exposed =( Window , Worker ), SecureContext ]
interface GPUSupportedFeatures {
readonly setlike < DOMString >;
};

NOTE: 

The type of the GPUSupportedFeatures set entries is DOMString to allow user
agents to gracefully handle valid GPUFeatureName s which are added in later revisions of the spec
but which the user agent has not been updated to recognize yet. If the set entries type was
GPUFeatureName the following code would throw an TypeError rather than reporting false :

Check for support of an unrecognized feature:

if ( adapter . features . has ( 'unknown-feature' )) { 
// Use unknown-feature 
} else { 
console . warn ( 'unknown-feature is not supported by this adapter.' ); 
} 

3.6.2.3. WGSLLanguageFeatures 

WGSLLanguageFeatures is the setlike interface of
navigator.gpu. wgslLanguageFeatures .
Its set entries are the string names of the WGSL language extensions 
supported by the implementation (regardless of the adapter or device). 

[ Exposed =( Window , Worker ), SecureContext ]
interface WGSLLanguageFeatures {
readonly setlike < DOMString >;
};

3.6.2.4. GPUAdapterInfo 

GPUAdapterInfo exposes various identifying information about an adapter. 

None of the members in GPUAdapterInfo are guaranteed to be populated with any particular value;
if no value is provided, the attribute will return the empty string "" . It is at the user
agent’s discretion which values to reveal, and it is likely that on some devices none of the values
will be populated. As such, applications must be able to handle any possible GPUAdapterInfo values,
including the absence of those values. 

The GPUAdapterInfo for an adapter is exposed via GPUAdapter.info 
and GPUDevice.adapterInfo ).
This info is immutable:
for a given adapter, each GPUAdapterInfo attribute will return the same value every time it’s accessed. 

Note: 
Though the GPUAdapterInfo attributes are immutable once accessed , an implementation may delay the decision on
what to expose for each attribute until the first time it is accessed. 

Note: 
Other GPUAdapter instances, even if they represent the same physical adapter, may expose
different values in GPUAdapterInfo .
However, they should expose the same values unless a specific
event has increased the amount of identifying information the page is allowed to access.
(No such events are defined by this specification.) 

For privacy considerations, see § 2.2.6 Adapter Identifiers .

[ Exposed =( Window , Worker ), SecureContext ]
interface GPUAdapterInfo {
readonly attribute DOMString vendor ;
readonly attribute DOMString architecture ;
readonly attribute DOMString device ;
readonly attribute DOMString description ;
readonly attribute unsigned long subgroupMinSize ;
readonly attribute unsigned long subgroupMaxSize ;
readonly attribute boolean isFallbackAdapter ;
};

GPUAdapterInfo has the following attributes: 

vendor , of type DOMString , readonly 

The name of the vendor of the adapter , if available. Empty string otherwise. 

architecture , of type DOMString , readonly 

The name of the family or class of GPUs the adapter belongs to, if available. Empty
string otherwise. 

device , of type DOMString , readonly 

A vendor-specific identifier for the adapter , if available. Empty string otherwise. 

Note: This is a value that represents the type of adapter. For example, it may be a
PCI device ID . It does not uniquely identify a given piece of
hardware like a serial number. 

description , of type DOMString , readonly 

A human readable string describing the adapter as reported by the driver, if available.
Empty string otherwise. 

Note: Because no formatting is applied to description attempting to parse
this value is not recommended. Applications which change their behavior based on the
GPUAdapterInfo , such as applying workarounds for known driver issues, should rely on the
other fields when possible. 

subgroupMinSize , of type unsigned long , readonly 

If the "subgroups" feature is supported, the minimum
supported subgroup size for the adapter . 

subgroupMaxSize , of type unsigned long , readonly 

If the "subgroups" feature is supported, the maximum
supported subgroup size for the adapter . 

isFallbackAdapter , of type boolean , readonly 

Whether the adapter is a fallback adapter . 

To create a new adapter info for a given adapter adapter , run the
following content timeline steps:

Let adapterInfo be a new GPUAdapterInfo . 

If the vendor is known, set adapterInfo . vendor to the name of
adapter ’s vendor as a normalized identifier string . To preserve privacy, the user
agent may instead set adapterInfo . vendor to the empty string or a
reasonable approximation of the vendor as a normalized identifier string . 

If |the architecture is known, set adapterInfo . architecture to a
normalized identifier string representing the family or class of adapters to which
adapter belongs. To preserve privacy, the user agent may instead set
adapterInfo . architecture to the empty string or a reasonable
approximation of the architecture as a normalized identifier string . 

If the device is known, set adapterInfo . device to a
normalized identifier string representing a vendor-specific identifier for adapter .
To preserve privacy, the user agent may instead set adapterInfo . device 
to to the empty string or a reasonable approximation of a vendor-specific identifier as a
normalized identifier string . 

If a description is known, set adapterInfo . description to a description
of the adapter as reported by the driver. To preserve privacy, the user agent may
instead set adapterInfo . description to the empty string or a
reasonable approximation of a description. 

If "subgroups" is supported, set subgroupMinSize 
to the smallest supported subgroup size. Otherwise, set this value to 4. 

Note: To preserve privacy, the user agent may choose to not support some features or provide values
for the property which do not distinguish different devices, but are still usable
(e.g. use the default value of 4 for all devices). 

If "subgroups" is supported, set subgroupMaxSize 
to the largest supported subgroup size. Otherwise, set this value to 128. 

Note: To preserve privacy, the user agent may choose to not support some features or provide values
for the property which do not distinguish different devices, but are still usable
(e.g. use the default value of 128 for all devices). 

Set adapterInfo . isFallbackAdapter to adapter . [[fallback]] . 

Return adapterInfo . 

A normalized identifier string is one that follows the following pattern:

[a-z0-9]+(-[a-z0-9]+)* 

Examples of valid normalized identifier strings include:

gpu 

3d 

0x3b2f 

next-gen 

series-x20-ultra 

3.7. Feature Detection

This section is non-normative. 

Fully implementing this specification requires implementation of everything it specifies, except
where otherwise stated (like § 3.6 Optional Capabilities ). 

However, since new "core" additions are added to this specification before being exposed by
implementations, many features are designed to be feature-detectable by applications: 

Interface support can be detected with typeof InterfaceName !== 'undefined' . 

Method and attribute support can be detected with 'itemName' in InterfaceName.prototype . 

New dictionary members, if they need to be detectable, generally document a specific
mechanism for feature detection. For example: 

unclippedDepth support is part of a device feature,
"depth-clip-control" . 

Canvas support for toneMapping is detected using
getConfiguration() . 

3.8. Extension Documents 

"Extension Documents" are additional documents which describe new functionality which is
non-normative and not part of the WebGPU/WGSL specifications .
They describe functionality that builds upon these specifications, often including one or more new
API feature flags and/or WGSL enable directives, or interactions with other draft
web specifications. 

WebGPU implementations must not expose extension functionality; doing so is a spec violation.
New functionality does not become part of the WebGPU standard until it is integrated
into the WebGPU specification (this document) and/or WGSL specification. 

3.9. Origin Restrictions 

WebGPU allows accessing image data stored in images, videos, and canvases.
Restrictions are imposed on the use of cross-domain media, because shaders can be used to
indirectly deduce the contents of textures which have been uploaded to the GPU. 

WebGPU disallows uploading an image source if it is not origin-clean . 

This also implies that the origin-clean flag for a
canvas rendered using WebGPU will never be set to false . 

For more information on issuing CORS requests for image and video elements, consult: 

HTML § 2.5.4 CORS settings attributes 

HTML § 4.8.3 The img element img 

HTML § 4.8.11 Media elements HTMLMediaElement 

3.10. Task Sources 

3.10.1. WebGPU Task Source 

WebGPU defines a new task source called the WebGPU task source .
It is used for the uncapturederror event and GPUDevice . lost . 

To queue a global task for GPUDevice device ,
with a series of steps steps on the content timeline :

Queue a global task on the WebGPU task source , with the global object that was used
to create device , and the steps steps . 

3.10.2. Automatic Expiry Task Source

WebGPU defines a new task source called the automatic expiry task source .
It is used for the automatic, timed expiry (destruction) of certain objects: 

GPUTexture s returned by getCurrentTexture() 

GPUExternalTexture s created from HTMLVideoElement s 

To queue an automatic expiry task 
with GPUDevice device and a series of steps steps on the content timeline :

Queue a global task on the automatic expiry task source , with the global object that
was used to create device , and the steps steps . 

Tasks from the automatic expiry task source should be processed with high priority; in
particular, once queued, they should run before user-defined (JavaScript) tasks. 

NOTE: 

This behavior is more predictable, and the strictness helps developers write more portable
applications by eagerly detecting incorrect assumptions about implicit lifetimes that may be
hard to detect. Developers are still strongly encouraged to test in multiple implementations.

Implementation note:
It is valid to implement a high-priority expiry "task" by instead inserting additional steps at
a fixed point inside the event loop processing model rather than running an actual task. 

3.11. Color Spaces and Encoding 

WebGPU does not provide color management. All values within WebGPU (such as texture elements)
are raw numeric values, not color-managed color values. 

WebGPU does interface with color-managed outputs (via GPUCanvasConfiguration ) and inputs
(via copyExternalImageToTexture() and importExternalTexture() ).
Thus, color conversion must be performed between the WebGPU numeric values and the external color values.
Each such interface point locally defines an encoding (color space, transfer function, and alpha
premultiplication) in which the WebGPU numeric values are to be interpreted. 

WebGPU allows all of the color spaces in the PredefinedColorSpace enum.
Note, each color space is defined over an extended range, as defined by the referenced CSS definitions,
to represent color values outside of its space (in both chrominance and luminance). 

NOTE: 

As described above, GPUTexture s are not color managed. This includes -srgb formats,
which despite their are not tagged with an sRGB color space (like those described by
PredefinedColorSpace and the CSS color spaces srgb and
srgb-linear ).

However, -srgb texture formats do have gamma-encoding/decoding properties which are
algorithmically close to those used for gamma encoding in "srgb" and
"display-p3" . For example, a fragment
shader can output an "sRGB-linear"-encoded (physically linear) color value into an -srgb 
format texture, which will gamma-encode the value when it is written.
Then, the value in the texture will be correctly encoded for use on a
"srgb" -tagged (approximately perceptually-linear) canvas. 

It is similarly possible to take advantage of these properties using
copyExternalImageToTexture() ; see its description for additional information. 

An out-of-gamut premultiplied RGBA value is one where any of the R/G/B channel values
exceeds the alpha channel value. For example, the premultiplied sRGB RGBA value [1.0, 0, 0, 0.5]
represents the (unpremultiplied) color [2, 0, 0] with 50% alpha, written rgb(srgb 2 0 0 / 50%) in CSS.
Just like any color value outside the sRGB color gamut, this is a well defined point in the extended color space
(except when alpha is 0, in which case there is no color).
However, when such values are output to a visible canvas, the result is undefined
(see GPUCanvasAlphaMode "premultiplied" ). 

3.11.1. Color Space Conversions 

A color is converted between spaces by translating its representation in one space to a
representation in another according to the definitions above. 

If the source value has fewer than 4 RGBA channels, the missing green/blue/alpha channels are set to
0, 0, 1 , respectively, before converting for color space/encoding and alpha premultiplication.
After conversion, if the destination needs fewer than 4 channels, the additional channels
are ignored. 

Note: 
Grayscale images generally represent RGB values (V, V, V) , or RGBA values (V, V, V, A) in their color space. 

Colors are not lossily clamped during conversion: converting from one color space to another
will result in values outside the range [0, 1] if the source color values were outside the range
of the destination color space’s gamut. For an sRGB destination, for example, this can occur if the
source is rgba16float, in a wider color space like Display-P3, or is premultiplied and contains
out-of-gamut values . 

Similarly, if the source value has a high bit depth (e.g. PNG with 16 bits per component) or
extended range (e.g. canvas with float16 storage), these colors are preserved through color space
conversion, with intermediate computations having at least the precision of the source. 

3.11.2. Color Space Conversion Elision 

If the source and destination of a color space/encoding conversion are the same, then conversion
is not necessary. In general, if any given step of the conversion is an identity function (no-op),
implementations should elide it, for performance. 

For optimal performance, applications should set their color space and encoding
options so that the number of necessary conversions is minimized throughout the process.
For various image sources of GPUCopyExternalImageSourceInfo : 

ImageBitmap : 

Premultiplication is controlled via premultiplyAlpha . 

Color space is controlled via colorSpaceConversion . 

2d canvas: 

Always premultiplied . 

Color space is controlled via the colorSpace context creation attribute. 

WebGL canvas: 

Premultiplication is controlled via the premultipliedAlpha option in WebGLContextAttributes . 

Color space is controlled via the WebGLRenderingContextBase ’s drawingBufferColorSpace state. 

Note: Check browser implementation support for these features before relying on them. 

3.12. Numeric conversions from JavaScript to WGSL 

Several parts of the WebGPU API ( pipeline-overridable constants and
render pass clear values) take numeric values from WebIDL ( double or float ) and convert
them to WGSL values ( bool , i32 , u32 , f32 , f16 ). 

To convert an IDL value idlValue of type double or float to WGSL type T ,
possibly throwing a TypeError , run the following device timeline steps:

Note: This TypeError is generated in the device timeline and never surfaced to JavaScript. 

Assert idlValue is a finite value, since it is not unrestricted double or unrestricted float . 

Let v be the ECMAScript Number resulting from ! converting idlValue to
an ECMAScript value . 

If T is bool 

Return the WGSL bool value corresponding to the result of ! converting v to
an IDL value of type boolean . 

Note: 
This algorithm is called after the conversion from an ECMAScript value to an IDL
double or float value. If the original ECMAScript value was a non-numeric,
non-boolean value like [] or {} , then the WGSL bool result may be different
than if the ECMAScript value had been converted to IDL boolean directly. 

If T is i32 

Return the WGSL i32 value corresponding to the result of ? converting v to
an IDL value of type [ EnforceRange ] long . 

If T is u32 

Return the WGSL u32 value corresponding to the result of ? converting v to
an IDL value of type [ EnforceRange ] unsigned long . 

If T is f32 

Return the WGSL f32 value corresponding to the result of ? converting v to
an IDL value of type float . 

If T is f16 

Let wgslF32 be the WGSL f32 value corresponding to the result of ? converting v to
an IDL value of type float . 

Return f16( wgslF32 ) , the result of ! converting the WGSL f32 value
to f16 as defined in WGSL floating point conversion . 

Note: As long as the value is in-range of f32 , no error is thrown, even if the
value is out-of-range of f16 . 

To convert a GPUColor color to a texel value of texture format format ,
possibly throwing a TypeError , run the following device timeline steps:

Note: This TypeError is generated in the device timeline and never surfaced to JavaScript. 

If the components of format ( assert they all have the same type) are: 

floating-point types or normalized types

Let T be f32 . 

signed integer types

Let T be i32 . 

unsigned integer types

Let T be u32 . 

Let wgslColor be a WGSL value of type vec4< T > , where the 4
components are the RGBA channels of color , each ? converted to WGSL type T . 

Convert wgslColor to format using the same conversion rules as the § 23.2.7 Output Merging 
step, and return the result. 

Note: 
For non-integer types, the exact choice of value is implementation-defined .
For normalized types, the value is clamped to the range of the type. 

Note: 
In other words, the value written will be as if it was written by a WGSL shader that
outputs the value represented as a vec4 of f32 , i32 , or u32 . 

4. Initialization 

4.1. navigator.gpu 

A GPU object is available in the Window and WorkerGlobalScope contexts through the
Navigator and WorkerNavigator interfaces respectively and is exposed via navigator.gpu : 

interface mixin NavigatorGPU {
[ SameObject , SecureContext ] readonly attribute GPU gpu ;
};
Navigator includes NavigatorGPU ;
WorkerNavigator includes NavigatorGPU ;

NavigatorGPU has the following attributes: 

gpu , of type GPU , readonly 

A global singleton providing top-level entry points like requestAdapter() . 

4.2. GPU 

GPU is the entry point to WebGPU. 

[ Exposed =( Window , Worker ), SecureContext ]
interface GPU {
Promise < GPUAdapter ?> requestAdapter ( optional GPURequestAdapterOptions options = {});
GPUTextureFormat getPreferredCanvasFormat ();
[ SameObject ] readonly attribute WGSLLanguageFeatures wgslLanguageFeatures ;
};

GPU has the following methods: 

requestAdapter(options) 

Requests an adapter from the user agent.
The user agent chooses whether to return an adapter, and, if so,
chooses according to the provided options. 

Called on: GPU this .

Arguments: 

Arguments for the GPU.requestAdapter(options) method. 

Parameter
Type
Nullable
Optional
Description

options 
GPURequestAdapterOptions 
✘ 
✔ 
Criteria used to select the adapter.

Returns: Promise < GPUAdapter ?> 

Content timeline steps: 

Let contentTimeline be the current Content timeline . 

Let promise be a new promise . 

Issue the initialization steps on the Device timeline of this . 

Return promise . 

Device timeline initialization steps :

All of the requirements in the following steps must be met. 

options . featureLevel must be
a feature level string . 

If any are unmet: 

Let adapter be null , issue the resolution steps 
on contentTimeline , and return. 

If options . featureLevel is "compatibility" : 

Set options . featureLevel to
"compatibility" if the user agent chooses
to support it, or "core" if not. 

Note: This doesn’t modify the JavaScript object passed by the application. 

Set adapter to either: 

A new adapter object chosen according to
the rules in § 4.2.2 Adapter Selection and the criteria in options ,
adhering to § 4.2.1 Adapter Capability Guarantees , with the capabilities
determined in an implementation-defined way by the user agent. 

null , if the user agent is unable to return an adapter, or makes an
implementation-defined choice not to return an adapter. 

If an adapter is returned, initialize its properties according to their
definitions. 

Set adapter . [[limits]] and adapter . [[features]] 
according to the supported capabilities of the adapter. 

If adapter meets the criteria of a fallback adapter set
adapter . [[fallback]] to true . Otherwise, set it to false . 

Set adapter . [[xrCompatible]] to
options . xrCompatible . 

Set adapter . [[default feature level]] to
options . featureLevel . 

Issue the resolution steps on contentTimeline . 

Content timeline resolution steps :

If adapter is not null : 

Resolve promise with a new GPUAdapter encapsulating adapter . 

Otherwise: 

Resolve promise with null . 

getPreferredCanvasFormat() 

Returns an optimal GPUTextureFormat for displaying 8-bit depth, standard dynamic range
content on this system. Must only return "rgba8unorm" or
"bgra8unorm" . 

The returned value can be passed as the format to
configure() calls on a GPUCanvasContext to ensure the associated
canvas is able to display its contents efficiently. 

Note: Canvases which are not displayed to the screen may or may not benefit from using this
format. 

Called on: GPU this.

Returns: GPUTextureFormat 

Content timeline steps: 

Return either "rgba8unorm" or
"bgra8unorm" , depending on which format is optimal for
displaying WebGPU canvases on this system. 

GPU has the following attributes: 

wgslLanguageFeatures , of type WGSLLanguageFeatures , readonly 

The names of supported WGSL language extensions .
Supported language extensions are automatically enabled. 

Adapters may expire at any time. Upon any change in the system’s state that could affect
the result of any requestAdapter() call, the user agent should expire all
previously-returned adapters . For example: 

A physical adapter is added/removed (via plug/unplug, driver update, hang recovery, etc.) 

The system’s power configuration has changed (laptop unplugged, power settings changed, etc.) 

Note: 
User agents may choose to expire adapters often, even when there has been no system
state change (e.g. seconds or minutes after the adapter was created).
This can help obfuscate real system state changes, and make developers more aware that calling
requestAdapter() again is always necessary before calling requestDevice() .
If an application does encounter this situation, standard device-loss recovery
handling should allow it to recover. 

Requesting a GPUAdapter with no hints:

const gpuAdapter = await navigator . gpu . requestAdapter (); 

4.2.1. Adapter Capability Guarantees 

Any GPUAdapter returned by requestAdapter() must provide the following guarantees: 

At least one of the following must be true: 

"texture-compression-bc" is supported. 

Both "texture-compression-etc2" and
"texture-compression-astc" are supported. 

If "texture-compression-bc-sliced-3d" 
is supported, then "texture-compression-bc" must be supported. 

If "texture-compression-astc-sliced-3d" 
is supported, then "texture-compression-astc" must be supported. 

All supported limits must be either the default value or better . 

All alignment-class limits must be powers of 2. 

maxBindingsPerBindGroup must be must be ≥
( max bindings per shader stage × max shader stages per pipeline ), where: 

max bindings per shader stage is
( maxSampledTexturesPerShaderStage +
maxSamplersPerShaderStage +
maxStorageBuffersPerShaderStage +
maxStorageTexturesPerShaderStage +
maxUniformBuffersPerShaderStage ). 

max shader stages per pipeline is 2 , because a
GPURenderPipeline supports both a vertex and fragment shader. 

Note: maxBindingsPerBindGroup does not reflect a fundamental limit;
implementations should raise it to conform to this requirement, rather than lowering the
other limits. 

maxBindGroups must be ≤ maxBindGroupsPlusVertexBuffers . 

maxVertexBuffers must be ≤ maxBindGroupsPlusVertexBuffers . 

minUniformBufferOffsetAlignment and
minStorageBufferOffsetAlignment must both be ≥ 32 bytes. 

Note: 32 bytes would be the alignment of vec4<f64> . See WebGPU Shading Language § 14.4.1 Alignment and Size . 

maxUniformBufferBindingSize must be ≤ maxBufferSize . 

maxStorageBufferBindingSize must be ≤ maxBufferSize . 

maxStorageBufferBindingSize must be a multiple of 4 bytes. 

maxVertexBufferArrayStride must be a multiple of 4 bytes. 

maxComputeWorkgroupSizeX must be ≤ maxComputeInvocationsPerWorkgroup . 

maxComputeWorkgroupSizeY must be ≤ maxComputeInvocationsPerWorkgroup . 

maxComputeWorkgroupSizeZ must be ≤ maxComputeInvocationsPerWorkgroup . 

maxComputeInvocationsPerWorkgroup must be ≤ maxComputeWorkgroupSizeX 
× maxComputeWorkgroupSizeY × maxComputeWorkgroupSizeZ . 

4.2.2. Adapter Selection 

GPURequestAdapterOptions 
provides hints to the user agent indicating what
configuration is suitable for the application. 

dictionary GPURequestAdapterOptions {
DOMString featureLevel = "core";
GPUPowerPreference powerPreference ;
boolean forceFallbackAdapter = false ;
boolean xrCompatible = false ;
};

enum GPUPowerPreference {
"low-power" ,
"high-performance" ,
};

GPURequestAdapterOptions has the following members: 

featureLevel , of type DOMString , defaulting to "core" 

Requests an adapter that supports at least a particular set of capabilities .
This influences the [[default feature level]] of devices created
from this adapter. The capabilities for each level are defined below, and the exact
steps are defined in requestAdapter() and " a new device ". 

If the implementation or system does not support all of the capabilities in the
requested feature level, requestAdapter() will return null . 

Note: 
Applications should typically make a single requestAdapter() call with the lowest
feature level they support, then inspect the adapter for additional capabilities they can
use optionally, and request those in requestDevice() . 

The allowed feature level string values are: 

"core" 

The following set of capabilities: 

The Default limits. 

"core-features-and-limits" . 

Note: 
Adapters with this [[default feature level]] may
conventionally be referred to as "Core-defaulting". 

"compatibility" 

The following set of capabilities: 

The Compatibility Mode Default limits. 

No features. (It excludes the "core-features-and-limits" feature.) 

If the implementation cannot enforce the stricter "Compatibility Mode"
validation rules, requestAdapter() will ignore this request and
treat it as a request for "core" . 

Note: 
Adapters with this [[default feature level]] may
conventionally be referred to as "Compatibility-defaulting". 

powerPreference , of type GPUPowerPreference 

Optionally provides a hint indicating what class of adapter should be selected from
the system’s available adapters. 

The value of this hint may influence which adapter is chosen, but it must not
influence whether an adapter is returned or not. 

Note: 
The primary utility of this hint is to influence which GPU is used in a multi-GPU system.
For instance, some laptops have a low-power integrated GPU and a high-performance
discrete GPU. This hint may also affect the power configuration of the selected GPU to
match the requested power preference. 

Note: 
Depending on the exact hardware configuration, such as battery status and attached displays
or removable GPUs, the user agent may select different adapters given the same power
preference.
Typically, given the same hardware configuration and state and
powerPreference , the user agent is likely to select the same adapter. 

It must be one of the following values: 

undefined (or not present)

Provides no hint to the user agent. 

"low-power" 

Indicates a request to prioritize power savings over performance. 

Note: 
Generally, content should use this if it is unlikely to be constrained by drawing
performance; for example, if it renders only one frame per second, draws only relatively
simple geometry with simple shaders, or uses a small HTML canvas element.
Developers are encouraged to use this value if their content allows, since it may
significantly improve battery life on portable devices. 

"high-performance" 

Indicates a request to prioritize performance over power consumption. 

Note: 
By choosing this value, developers should be aware that, for devices created on the
resulting adapter, user agents are more likely to force device loss, in order to save
power by switching to a lower-power adapter.
Developers are encouraged to only specify this value if they believe it is absolutely
necessary, since it may significantly decrease battery life on portable devices. 

forceFallbackAdapter , of type boolean , defaulting to false 

When set to true indicates that only a fallback adapter may be returned. If the user
agent does not support a fallback adapter , will cause requestAdapter() to
resolve to null . 

Note: 
requestAdapter() may still return a fallback adapter if
forceFallbackAdapter is set to false and either no
other appropriate adapter is available or the user agent chooses to return a
fallback adapter . Developers that wish to prevent their applications from running on
fallback adapters should check the
info . isFallbackAdapter attribute prior
to requesting a GPUDevice . 

xrCompatible , of type boolean , defaulting to false 

When set to true indicates that the best adapter for rendering to a WebXR session 
must be returned. If the user agent or system does not support WebXR sessions then
adapter selection may ignore this value. 

Note: 
If xrCompatible is not set to true when the adapter is
requested, GPUDevice s created from the adapter cannot be used to render for
WebXR sessions . 

Requesting a "high-performance" GPUAdapter :

const gpuAdapter = await navigator . gpu . requestAdapter ({ 
powerPreference : 'high-performance' 
}); 

4.3. GPUAdapter 

A GPUAdapter encapsulates an adapter ,
and describes its capabilities ( features and limits ). 

To get a GPUAdapter , use requestAdapter() . 

[ Exposed =( Window , Worker ), SecureContext ]
interface GPUAdapter {
[ SameObject ] readonly attribute GPUSupportedFeatures features ;
[ SameObject ] readonly attribute GPUSupportedLimits limits ;
[ SameObject ] readonly attribute GPUAdapterInfo info ;

Promise < GPUDevice > requestDevice ( optional GPUDeviceDescriptor descriptor = {});
};

GPUAdapter has the following immutable properties 

features , of type GPUSupportedFeatures , readonly 

The set of values in this . [[adapter]] . [[features]] . 

limits , of type GPUSupportedLimits , readonly 

The limits in this . [[adapter]] . [[limits]] . 

info , of type GPUAdapterInfo , readonly 

Information about the physical adapter underlying this GPUAdapter . 

For a given GPUAdapter , the GPUAdapterInfo values exposed are constant over time. 

The same object is returned each time. To create that object for the first time: 

Called on: GPUAdapter this .

Returns: GPUAdapterInfo 

Content timeline steps: 

Return a new adapter info for this . [[adapter]] . 

[[adapter]] , of type adapter , readonly

The adapter to which this GPUAdapter refers. 

GPUAdapter has the following methods: 

requestDevice(descriptor) 

Requests a device from the adapter . 

This is a one-time action: if a device is returned successfully,
the adapter becomes "consumed" . 

Called on: GPUAdapter this .

Arguments: 

Arguments for the GPUAdapter.requestDevice(descriptor) method. 

Parameter
Type
Nullable
Optional
Description

descriptor 
GPUDeviceDescriptor 
✘ 
✔ 
Description of the GPUDevice to request.

Returns: Promise < GPUDevice > 

Content timeline steps: 

Let contentTimeline be the current Content timeline . 

Let promise be a new promise . 

Let adapter be this . [[adapter]] . 

Issue the initialization steps to the Device timeline of this . 

Return promise . 

Device timeline initialization steps :

If any of the following requirements are unmet: 

The set of values in descriptor . requiredFeatures 
must be a subset of those in adapter . [[features]] . 

Then issue the following steps on contentTimeline 
and return: 

Content timeline steps:

Reject promise with a TypeError . 

Note: This is the same error that is produced if a feature name isn’t known
by the browser at all (in its GPUFeatureName definition).
This converges the behavior when the browser doesn’t support a feature
with the behavior when a particular adapter doesn’t support a feature. 

All of the requirements in the following steps must be met. 

adapter . [[state]] must not be "consumed" . 

For each [ key , value ] in descriptor . requiredLimits 
for which value is not undefined : 

key must be the name of a member of supported limits . 

value must be no better than adapter . [[limits]] [ key ]. 

If key ’s class is alignment ,
value must be a power of 2 less than 2 32 . 

Note: 
User agents should consider issuing developer-visible warnings when
key is not recognized, even when value is undefined . 

If any are unmet, issue the following steps on contentTimeline 
and return: 

Content timeline steps:

Reject promise with an OperationError . 

If adapter . [[state]] is "expired" 
or the user agent otherwise cannot fulfill the request: 

Let device be a new device . 

Lose the device ( device , "unknown" ). 

Assert adapter . [[state]] is "expired" . 

Note: 
User agents should consider issuing developer-visible warnings in
most or all cases when this occurs. Applications should perform
reinitialization logic starting with requestAdapter() . 

Otherwise: 

Let device be the result of creating a new device from adapter with descriptor . 

Expire adapter . 

Issue the subsequent steps on contentTimeline . 

Content timeline steps:

Let gpuDevice be a new GPUDevice instance. 

Set gpuDevice . [[device]] to device . 

Set device . [[content device]] to gpuDevice . 

Set gpuDevice . label to descriptor . label . 

Resolve promise with gpuDevice . 

Note: 
If the device is already lost because the adapter could not fulfill the request,
device . lost has already resolved before promise resolves. 

Requesting a GPUDevice with default features and limits:

const gpuAdapter = await navigator . gpu . requestAdapter (); 
const gpuDevice = await gpuAdapter . requestDevice (); 

4.3.1. GPUDeviceDescriptor 

GPUDeviceDescriptor describes a device request. 

dictionary GPUDeviceDescriptor 
: GPUObjectDescriptorBase {
sequence < GPUFeatureName > " href="#dom-gpudevicedescriptor-requiredfeatures" id="ref-for-dom-gpudevicedescriptor-requiredfeatures②"> requiredFeatures = [];
record < DOMString , ( GPUSize64 or undefined )> " href="#dom-gpudevicedescriptor-requiredlimits" id="ref-for-dom-gpudevicedescriptor-requiredlimits②"> requiredLimits = {};
GPUQueueDescriptor defaultQueue = {};
};

GPUDeviceDescriptor has the following members: 

requiredFeatures , of type sequence< GPUFeatureName >, defaulting to [] 

Specifies the features that are required by the device request.
The request will fail if the adapter cannot provide these features. 

Exactly the specified set of features, and no more or less, will be allowed in validation
of API calls on the resulting device. 

requiredLimits , of type record<DOMString, (GPUSize64 or undefined)> , defaulting to {} 

Specifies the limits that are required by the device request.
The request will fail if the adapter cannot provide these limits. 

Each key with a non- undefined value must be the name of a member of supported limits . 

API calls on the resulting device perform validation according to the exact limits of the
device (not the adapter; see § 3.6.2 Limits ). 

defaultQueue , of type GPUQueueDescriptor , defaulting to {} 

The descriptor for the default GPUQueue . 

Requesting a GPUDevice with the "texture-compression-astc" feature if supported:

const gpuAdapter = await navigator . gpu . requestAdapter (); 

const requiredFeatures = []; 
if ( gpuAdapter . features . has ( 'texture-compression-astc' )) { 
requiredFeatures . push ( 'texture-compression-astc' ) 
} 

const gpuDevice = await gpuAdapter . requestDevice ({ 
requiredFeatures
}); 

Requesting a GPUDevice with a higher maxColorAttachmentBytesPerSample limit:

const gpuAdapter = await navigator . gpu . requestAdapter (); 

if ( gpuAdapter . limits . maxColorAttachmentBytesPerSample < 64 ) { 
// When the desired limit isn't supported, take action to either fall back to a code 
// path that does not require the higher limit or notify the user that their device 
// does not meet minimum requirements. 
} 

// Request higher limit of max color attachments bytes per sample. 
const gpuDevice = await gpuAdapter . requestDevice ({ 
requiredLimits : { maxColorAttachmentBytesPerSample : 64 }, 
}); 

4.3.1.1. GPUFeatureName 

Each GPUFeatureName identifies a set of functionality which, if available,
allows additional usages of WebGPU that would have otherwise been invalid. 

enum GPUFeatureName {
"core-features-and-limits" ,
"depth-clip-control" ,
"depth32float-stencil8" ,
"texture-compression-bc" ,
"texture-compression-bc-sliced-3d" ,
"texture-compression-etc2" ,
"texture-compression-astc" ,
"texture-compression-astc-sliced-3d" ,
"timestamp-query" ,
"indirect-first-instance" ,
"shader-f16" ,
"rg11b10ufloat-renderable" ,
"bgra8unorm-storage" ,
"float32-filterable" ,
"float32-blendable" ,
"clip-distances" ,
"dual-source-blending" ,
"subgroups" ,
"texture-formats-tier1" ,
"texture-formats-tier2" ,
"primitive-index" ,
"texture-component-swizzle" ,
};

4.4. GPUDevice 

A GPUDevice encapsulates a device and exposes
the functionality of that device. 

GPUDevice is the top-level interface through which WebGPU interfaces are created. 

To get a GPUDevice , use requestDevice() . 

[ Exposed =( Window , Worker ), SecureContext ]
interface GPUDevice : EventTarget {
[ SameObject ] readonly attribute GPUSupportedFeatures features ;
[ SameObject ] readonly attribute GPUSupportedLimits limits ;
[ SameObject ] readonly attribute GPUAdapterInfo adapterInfo ;

[ SameObject ] readonly attribute GPUQueue queue ;

undefined destroy ();

GPUBuffer createBuffer ( GPUBufferDescriptor descriptor );
GPUTexture createTexture ( GPUTextureDescriptor descriptor );
GPUSampler createSampler ( optional GPUSamplerDescriptor descriptor = {});
GPUExternalTexture importExternalTexture ( GPUExternalTextureDescriptor descriptor );

GPUBindGroupLayout createBindGroupLayout ( GPUBindGroupLayoutDescriptor descriptor );
GPUPipelineLayout createPipelineLayout ( GPUPipelineLayoutDescriptor descriptor );
GPUBindGroup createBindGroup ( GPUBindGroupDescriptor descriptor );

GPUShaderModule createShaderModule ( GPUShaderModuleDescriptor descriptor );
GPUComputePipeline createComputePipeline ( GPUComputePipelineDescriptor descriptor );
GPURenderPipeline createRenderPipeline ( GPURenderPipelineDescriptor descriptor );
Promise < GPUComputePipeline > createComputePipelineAsync ( GPUComputePipelineDescriptor descriptor );
Promise < GPURenderPipeline > createRenderPipelineAsync ( GPURenderPipelineDescriptor descriptor );

GPUCommandEncoder createCommandEncoder ( optional GPUCommandEncoderDescriptor descriptor = {});
GPURenderBundleEncoder createRenderBundleEncoder ( GPURenderBundleEncoderDescriptor descriptor );

GPUQuerySet createQuerySet ( GPUQuerySetDescriptor descriptor );
};
GPUDevice includes GPUObjectBase ;

GPUDevice has the following immutable properties : 

features , of type GPUSupportedFeatures , readonly 

A set containing the GPUFeatureName values of the features
supported by the device ( [[device]] . [[features]] ). 

limits , of type GPUSupportedLimits , readonly 

The limits supported by the device ( [[device]] . [[limits]] ). 

queue , of type GPUQueue , readonly 

The primary GPUQueue for this device. 

adapterInfo , of type GPUAdapterInfo , readonly 

Information about the physical adapter which created the device that this GPUDevice refers to. 

For a given GPUDevice , the GPUAdapterInfo values exposed are constant over time. 

The same object is returned each time. To create that object for the first time: 

Called on: GPUDevice this .

Returns: GPUAdapterInfo 

Content timeline steps: 

Return a new adapter info for this . [[device]] . [[adapter]] . 

The [[device]] for a GPUDevice is the device that the GPUDevice refers
to. 

GPUDevice has the following methods: 

destroy() 

Destroys the device , preventing further operations on it.
Outstanding asynchronous operations will fail. 

Note: It is valid to destroy a device multiple times. 

Called on: GPUDevice this .

Content timeline steps: 

unmap() all GPUBuffer s from this device. 

Issue the subsequent steps on the Device timeline of this . 

Lose the device ( this . [[device]] ,
"destroyed" ). 

Note: Since no further operations can be enqueued on this device, implementations can abort
outstanding asynchronous operations immediately and free resource allocations, including
mapped memory that was just unmapped. 

A GPUDevice ’s allowed buffer usages are:

Always allowed:
MAP_READ ,
MAP_WRITE ,
COPY_SRC ,
COPY_DST ,
INDEX ,
VERTEX ,
UNIFORM ,
STORAGE ,
INDIRECT ,
QUERY_RESOLVE 

A GPUDevice ’s allowed texture usages are:

Always allowed:
COPY_SRC ,
COPY_DST ,
TEXTURE_BINDING ,
STORAGE_BINDING ,
RENDER_ATTACHMENT ,
TRANSIENT_ATTACHMENT 

4.5. Example 

A more robust example of requesting a GPUAdapter and GPUDevice with error handling:

let gpuDevice = null ; 

async function initializeWebGPU () { 
// Check to ensure the user agent supports WebGPU. 
if ( ! ( 'gpu' in navigator )) { 
console . error ( "User agent doesn't support WebGPU." ); 
return false ; 
} 

// Request an adapter. 
const gpuAdapter = await navigator . gpu . requestAdapter (); 

// requestAdapter may resolve with null if no suitable adapters are found. 
if ( ! gpuAdapter ) { 
console . error ( 'No WebGPU adapters found.' ); 
return false ; 
} 

// Request a device. 
// Note that the promise will reject if invalid options are passed to the optional 
// dictionary. To avoid the promise rejecting always check any features and limits 
// against the adapters features and limits prior to calling requestDevice(). 
gpuDevice = await gpuAdapter . requestDevice (); 

// requestDevice will never return null, but if a valid device request can't be 
// fulfilled for some reason it may resolve to a device which has already been lost. 
// Additionally, devices can be lost at any time after creation for a variety of reasons 
// (ie: browser resource management, driver updates), so it's a good idea to always 
// handle lost devices gracefully. 
gpuDevice . lost . then (( info ) => { 
console . error ( `WebGPU device was lost: ${ info . message } ` ); 

gpuDevice = null ; 

// Many causes for lost devices are transient, so applications should try getting a 
// new device once a previous one has been lost unless the loss was caused by the 
// application intentionally destroying the device. Note that any WebGPU resources 
// created with the previous device (buffers, textures, etc) will need to be 
// re-created with the new one. 
if ( info . reason != 'destroyed' ) { 
initializeWebGPU (); 
} 
}); 

onWebGPUInitialized (); 

return true ; 
} 

function onWebGPUInitialized () { 
// Begin creating WebGPU resources here... 
} 

initializeWebGPU (); 

5. Buffers 

5.1. GPUBuffer 

A GPUBuffer represents a block of memory that can be used in GPU operations.
Data is stored in linear layout, meaning that each byte of the allocation can be
addressed by its offset from the start of the GPUBuffer , subject to alignment
restrictions depending on the operation. Some GPUBuffers can be
mapped which makes the block of memory accessible via an ArrayBuffer called
its mapping. 

GPUBuffer s are created via createBuffer() .
Buffers may be mappedAtCreation . 

[ Exposed =( Window , Worker ), SecureContext ]
interface GPUBuffer {
readonly attribute GPUSize64Out size ;
readonly attribute GPUFlagsConstant usage ;

readonly attribute GPUBufferMapState mapState ;

Promise < undefined > mapAsync ( GPUMapModeFlags mode , optional GPUSize64 offset = 0, optional GPUSize64 size );
ArrayBuffer getMappedRange ( optional GPUSize64 offset = 0, optional GPUSize64 size );
undefined unmap ();

undefined destroy ();
};
GPUBuffer includes GPUObjectBase ;

enum GPUBufferMapState {
"unmapped" ,
"pending" ,
"mapped" ,
};

GPUBuffer has the following immutable properties : 

size , of type GPUSize64Out , readonly 

The length of the GPUBuffer allocation in bytes. 

usage , of type GPUFlagsConstant , readonly 

The allowed usages for this GPUBuffer . 

GPUBuffer has the following content timeline properties : 

mapState , of type GPUBufferMapState , readonly 

The current GPUBufferMapState of the buffer: 

"unmapped" 

The buffer is not mapped for use by this . getMappedRange() . 

"pending" 

A mapping of the buffer has been requested, but is pending.
It may succeed, or fail validation in mapAsync() . 

"mapped" 

The buffer is mapped and this . getMappedRange() may be used. 

The getter steps are: 

Content timeline steps:

If this . [[mapping]] is not null ,
return "mapped" . 

If this . [[pending_map]] is not null ,
return "pending" . 

Return "unmapped" . 

[[pending_map]] , of type Promise <void> or null , initially null 

The Promise returned by the currently-pending mapAsync() call. 

There is never more than one pending map, because mapAsync() 
will refuse immediately if a request is already in flight. 

[[mapping]] , of type active buffer mapping or null , initially null 

Set if and only if the buffer is currently mapped for use by getMappedRange() .
Null otherwise (even if there is a [[pending_map]] ). 

An active buffer mapping is a structure with the following fields: 

data , of type Data Block 

The mapping for this GPUBuffer . This data is accessed through ArrayBuffer s
which are views onto this data, returned by getMappedRange() and
stored in views . 

mode , of type GPUMapModeFlags 

The GPUMapModeFlags of the map, as specified in the corresponding call to
mapAsync() or createBuffer() . 

range , of type tuple [ unsigned long long , unsigned long long ]

The range of this GPUBuffer that is mapped. 

views , of type list < ArrayBuffer >

The ArrayBuffer s returned via getMappedRange() to the application.
They are tracked so they can be detached when unmap() is called. 

To initialize an active buffer mapping with mode mode and
range range , run the following content timeline steps:

Let size be range [1] - range [0]. 

Let data be ? CreateByteDataBlock ( size ). 

NOTE: 

This may result in a RangeError being thrown.
For consistency and predictability:

For any size at which new ArrayBuffer() would succeed at a given moment,
this allocation should succeed at that moment. 

For any size at which new ArrayBuffer() deterministically throws a
RangeError , this allocation should as well. 

Return an active buffer mapping with: 

data set to data . 

mode set to mode . 

range set to range . 

views set to [] . 

Mapping and unmapping a buffer. 

Failing to map a buffer. 

GPUBuffer has the following device timeline properties : 

[[internal state]] 

The current internal state of the buffer: 

" available "

The buffer can be used in queue operations (unless it is invalid ). 

" unavailable "

The buffer cannot be used in queue operations due to being mapped. 

" destroyed "

The buffer cannot be used in any operations due to being destroy() ed. 

5.1.1. GPUBufferDescriptor 

dictionary GPUBufferDescriptor 
: GPUObjectDescriptorBase {
required GPUSize64 size ;
required GPUBufferUsageFlags usage ;
boolean mappedAtCreation = false ;
};

GPUBufferDescriptor has the following members: 

size , of type GPUSize64 

The size of the buffer in bytes. 

usage , of type GPUBufferUsageFlags 

The allowed usages for the buffer. 

mappedAtCreation , of type boolean , defaulting to false 

If true creates the buffer in an already mapped state, allowing
getMappedRange() to be called immediately. It is valid to set
mappedAtCreation to true even if usage 
does not contain MAP_READ or MAP_WRITE . This can be
used to set the buffer’s initial data. 

Guarantees that even if the buffer creation eventually fails, it will still appear as if the
mapped range can be written/read to until it is unmapped. 

5.1.2. Buffer Usages 

typedef [ EnforceRange ] unsigned long GPUBufferUsageFlags ;
[ Exposed =( Window , Worker ), SecureContext ]
namespace GPUBufferUsage {
const GPUFlagsConstant MAP_READ = 0x0001;
const GPUFlagsConstant MAP_WRITE = 0x0002;
const GPUFlagsConstant COPY_SRC = 0x0004;
const GPUFlagsConstant COPY_DST = 0x0008;
const GPUFlagsConstant INDEX = 0x0010;
const GPUFlagsConstant VERTEX = 0x0020;
const GPUFlagsConstant UNIFORM = 0x0040;
const GPUFlagsConstant STORAGE = 0x0080;
const GPUFlagsConstant INDIRECT = 0x0100;
const GPUFlagsConstant QUERY_RESOLVE = 0x0200;
};

The GPUBufferUsage flags determine how a GPUBuffer may be used after its creation: 

MAP_READ 

The buffer can be mapped for reading. (Example: calling mapAsync() with
GPUMapMode.READ ) 

May only be combined with COPY_DST . 

MAP_WRITE 

The buffer can be mapped for writing. (Example: calling mapAsync() with
GPUMapMode.WRITE ) 

May only be combined with COPY_SRC . 

COPY_SRC 

The buffer can be used as the source of a copy operation. (Examples: as the source 
argument of a copyBufferToBuffer() or
copyBufferToTexture() call.) 

COPY_DST 

The buffer can be used as the destination of a copy or write operation. (Examples: as the
destination argument of a copyBufferToBuffer() or
copyTextureToBuffer() call, or as the target of a
writeBuffer() call.) 

INDEX 

The buffer can be used as an index buffer. (Example: passed to
setIndexBuffer() .) 

VERTEX 

The buffer can be used as a vertex buffer. (Example: passed to
setVertexBuffer() .) 

UNIFORM 

The buffer can be used as a uniform buffer. (Example: as a bind group entry for a
GPUBufferBindingLayout with a
buffer . type of
"uniform" .) 

STORAGE 

The buffer can be used as a storage buffer. (Example: as a bind group entry for a
GPUBufferBindingLayout with a
buffer . type of
"storage" or "read-only-storage" .) 

INDIRECT 

The buffer can be used as to store indirect command arguments. (Examples: as the
indirectBuffer argument of a drawIndirect() or
dispatchWorkgroupsIndirect() call.) 

QUERY_RESOLVE 

The buffer can be used to capture query results. (Example: as the destination argument of
a resolveQuerySet() call.) 

5.1.3. Buffer Creation 

createBuffer(descriptor) 

Creates a GPUBuffer . 

Called on: GPUDevice this .

Arguments: 

Arguments for the GPUDevice.createBuffer(descriptor) method. 

Parameter
Type
Nullable
Optional
Description

descriptor 
GPUBufferDescriptor 
✘ 
✘ 
Description of the GPUBuffer to create.

Returns: GPUBuffer 

Content timeline steps: 

Let b be ! create a new WebGPU object ( this , GPUBuffer , descriptor ). 

Set b . size to descriptor . size . 

Set b . usage to descriptor . usage . 

If descriptor . mappedAtCreation is true : 

If descriptor . size is not a multiple of 4,
throw a RangeError . 

Set b . [[mapping]] to
? initialize an active buffer mapping with mode WRITE 
and range [0, descriptor . size ] . 

Issue the initialization steps on the Device timeline of this . 

Return b . 

Device timeline initialization steps :

If any of the following requirements are unmet,
generate a validation error , invalidate b and return. 

this must not be lost . 

descriptor . usage must not be 0. 

descriptor . usage must be a subset of the
allowed buffer usages for this . 

If descriptor . usage contains MAP_READ : 

descriptor . usage must contain no other flags
except COPY_DST . 

If descriptor . usage contains MAP_WRITE : 

descriptor . usage must contain no other flags
except COPY_SRC . 

If descriptor . size must be ≤
this . [[device]] . [[limits]] . maxBufferSize . 

Note: If buffer creation fails, and descriptor . mappedAtCreation is false ,
any calls to mapAsync() will reject, so any resources allocated to enable mapping can
and may be discarded or recycled. 

If descriptor . mappedAtCreation is true : 

Set b . [[internal state]] to " unavailable ". 

Otherwise: 

Set b . [[internal state]] to " available ". 

Create a device allocation for b where each byte is zero. 

If the allocation fails without side-effects,
generate an out-of-memory error , invalidate b , and return. 

Creating a 128 byte uniform buffer that can be written into:

const buffer = gpuDevice . createBuffer ({ 
size : 128 , 
usage : GPUBufferUsage . UNIFORM | GPUBufferUsage . COPY_DST
}); 

5.1.4. Buffer Destruction 

An application that no longer requires a GPUBuffer can choose to lose
access to it before garbage collection by calling destroy() . Destroying a buffer also
unmaps it, freeing any memory allocated for the mapping. 

Note: This allows the user agent to reclaim the GPU memory associated with the GPUBuffer 
once all previously submitted operations using it are complete. 

GPUBuffer has the following methods: 

destroy() 

Destroys the GPUBuffer . 

Note: It is valid to destroy a buffer multiple times. 

Called on: GPUBuffer this .

Returns: undefined 

Content timeline steps: 

Call this . unmap() . 

Issue the subsequent steps on the Device timeline of
this . [[device]] . 

Device timeline steps:

Set this . [[internal state]] to
" destroyed ". 

Note: Since no further operations can be enqueued using this buffer, implementations can
free resource allocations, including mapped memory that was just unmapped. 

5.2. Buffer Mapping 

An application can request to map a GPUBuffer so that they can access its
content via ArrayBuffer s that represent part of the GPUBuffer ’s
allocations. Mapping a GPUBuffer is requested asynchronously with
mapAsync() so that the user agent can ensure the GPU
finished using the GPUBuffer before the application can access its content.
A mapped GPUBuffer 
cannot be used by the GPU and must be unmapped using unmap() before
work using it can be submitted to the Queue timeline . 

Once the GPUBuffer is mapped, the application can synchronously ask for access
to ranges of its content with getMappedRange() .
The returned ArrayBuffer can only be detached by unmap() 
(directly, or via GPUBuffer . destroy() or GPUDevice . destroy() ),
and cannot be transferred .
A TypeError is thrown by any other operation that attempts to do so. 

typedef [ EnforceRange ] unsigned long GPUMapModeFlags ;
[ Exposed =( Window , Worker ), SecureContext ]
namespace GPUMapMode {
const GPUFlagsConstant READ = 0x0001;
const GPUFlagsConstant WRITE = 0x0002;
};

The GPUMapMode flags determine how a GPUBuffer is mapped when calling
mapAsync() : 

READ 

Only valid with buffers created with the MAP_READ usage. 

Once the buffer is mapped, calls to getMappedRange() will return an
ArrayBuffer containing the buffer’s current values. Changes to the returned
ArrayBuffer will be discarded after unmap() is called. 

WRITE 

Only valid with buffers created with the MAP_WRITE usage. 

Once the buffer is mapped, calls to getMappedRange() will return an
ArrayBuffer containing the buffer’s current values. Changes to the returned
ArrayBuffer will be stored in the GPUBuffer after unmap() is called. 

Note: Since the MAP_WRITE buffer usage may only be combined with the
COPY_SRC buffer usage, mapping for writing can never return values
produced by the GPU, and the returned ArrayBuffer will only ever contain the default
initialized data (zeros) or data written by the webpage during a previous mapping. 

GPUBuffer has the following methods: 

mapAsync(mode, offset, size) 

Maps the given range of the GPUBuffer and resolves the returned Promise when the
GPUBuffer ’s content is ready to be accessed with getMappedRange() . 

The resolution of the returned Promise only indicates that the buffer has been mapped.
It does not guarantee the completion of any other operations visible to the content timeline ,
and in particular does not imply that any other Promise returned from
onSubmittedWorkDone() or mapAsync() on other GPUBuffer s
have resolved. 

The resolution of the Promise returned from onSubmittedWorkDone() 
does imply the completion of
mapAsync() calls made prior to that call,
on GPUBuffer s last used exclusively on that queue. 

Called on: GPUBuffer this .

Arguments: 

Arguments for the GPUBuffer.mapAsync(mode, offset, size) method. 

Parameter
Type
Nullable
Optional
Description

mode 
GPUMapModeFlags 
✘ 
✘ 
Whether the buffer should be mapped for reading or writing.

offset 
GPUSize64 
✘ 
✔ 
Offset in bytes into the buffer to the start of the range to map.

size 
GPUSize64 
✘ 
✔ 
Size in bytes of the range to map.

Returns: Promise < undefined > 

Content timeline steps: 

Let contentTimeline be the current Content timeline . 

If this . mapState is not "unmapped" : 

Issue the early-reject steps on the Device timeline of
this . [[device]] . 

Return a promise rejected with OperationError . 

Let p be a new Promise . 

Set this . [[pending_map]] to p . 

Issue the validation steps on the Device timeline of
this . [[device]] . 

Return p . 

Device timeline early-reject steps :

Generate a validation error . 

Return. 

Device timeline validation steps :

If size is undefined : 

Let rangeSize be max(0, this . size - offset ). 

Otherwise: 

Let rangeSize be size . 

If any of the following conditions are unsatisfied: 

this must be valid . 

Set deviceLost to true . 

Issue the map failure steps on
contentTimeline . 

Return. 

If any of the following conditions are unsatisfied: 

this . [[internal state]] is " available ". 

offset is a multiple of 8. 

rangeSize is a multiple of 4. 

offset + rangeSize ≤ this . size 

mode contains only bits defined in GPUMapMode . 

mode contains exactly one of READ or WRITE . 

If mode contains READ then this . usage must contain MAP_READ . 

If mode contains WRITE then this . usage must contain MAP_WRITE . 

Then: 

Set deviceLost to false . 

Issue the map failure steps on
contentTimeline . 

Generate a validation error . 

Return. 

Set this . [[internal state]] to " unavailable ". 

Note: Since the buffer is mapped, its contents cannot change between this step and unmap() . 

When either of the following events occur (whichever comes first),
or if either has already occurred: 

The device timeline becomes informed of the completion of an unspecified
queue timeline point: 

after the completion of
currently-enqueued operations that use this 

and no later than the completion of
all currently-enqueued operations 
(regardless of whether they use this ). 

this . [[device]] becomes lost . 

Then issue the subsequent steps on the device timeline of this . [[device]] . 

Device timeline steps:

Set deviceLost to true if this . [[device]] is lost ,
and false otherwise. 

Note: The device could have been lost between the previous block of steps and this one. 

If deviceLost : 

Issue the map failure steps on
contentTimeline . 

Otherwise: 

Let internalStateAtCompletion be this . [[internal state]] . 

Note: If, and only if, at this point the buffer has become " available "
again due to an unmap() call, then [[pending_map]] != p below,
so mapping will not succeed in the steps below. 

Let dataForMappedRegion be the contents of this starting at offset offset , for rangeSize bytes. 

Issue the map success steps on the
contentTimeline . 

Content timeline map success steps :

If this . [[pending_map]] != p : 

Note: The map has been cancelled by unmap() . 

Assert p is rejected. 

Return. 

Assert p is pending. 

Assert internalStateAtCompletion is " unavailable ". 

Let mapping be initialize an active buffer mapping 
with mode mode and range [ offset , offset + rangeSize ] . 

If this allocation fails: 

Set this . [[pending_map]] to null ,
and reject p with a RangeError . 

Return. 

Set the content of mapping . data to dataForMappedRegion . 

Set this . [[mapping]] to mapping . 

Set this . [[pending_map]] to null ,
and resolve p . 

Content timeline map failure steps :

If this . [[pending_map]] != p : 

Note: The map has been cancelled by unmap() . 

Assert p is already rejected. 

Return. 

Assert p is still pending. 

Set this . [[pending_map]] to null . 

If deviceLost : 

Reject p with an AbortError . 

Note: This is the same error type produced by cancelling the map using
unmap() . 

Otherwise: 

Reject p with an OperationError . 

getMappedRange(offset, size) 

Returns an ArrayBuffer with the contents of the GPUBuffer in the given mapped range. 

Called on: GPUBuffer this .

Arguments: 

Arguments for the GPUBuffer.getMappedRange(offset, size) method. 

Parameter
Type
Nullable
Optional
Description

offset 
GPUSize64 
✘ 
✔ 
Offset in bytes into the buffer to return buffer contents from.

size 
GPUSize64 
✘ 
✔ 
Size in bytes of the ArrayBuffer to return.

Returns: ArrayBuffer 

Content timeline steps: 

If size is missing: 

Let rangeSize be max(0, this . size - offset ). 

Otherwise, let rangeSize be size . 

If any of the following conditions are unsatisfied, throw an OperationError and return. 

this . [[mapping]] is not null . 

offset is a multiple of 8. 

rangeSize is a multiple of 4. 

offset ≥ this . [[mapping]] . range [0]. 

offset + rangeSize ≤ this . [[mapping]] . range [1]. 

[ offset , offset + rangeSize ) does not overlap another range in
this . [[mapping]] . views . 

Note: It is always valid to get mapped ranges of a GPUBuffer that is
mappedAtCreation , even if it is invalid , because
the Content timeline might not know it is invalid. 

Let data be this . [[mapping]] . data . 

Let view be ! create an ArrayBuffer of size rangeSize ,
but with its pointer mutably referencing the content of data at offset
( offset - [[mapping]] . range [0]). 

Note: A RangeError cannot be thrown here, because the data has already
been allocated during mapAsync() or createBuffer() . 

Set view . [[ArrayBufferDetachKey]] to "WebGPUBufferMapping". 

Note: This causes a TypeError to be thrown if an attempt is made to
DetachArrayBuffer , except by unmap() . 

Append view to this . [[mapping]] . views . 

Return view . 

Note: User agents should consider issuing a developer-visible warning if
getMappedRange() succeeds without having checked the status of
the map, by waiting for mapAsync() to succeed, querying a
mapState of "mapped" , or waiting for a
later onSubmittedWorkDone() call to succeed. 

unmap() 

Unmaps the mapped range of the GPUBuffer and makes its contents available for use by the
GPU again. 

Called on: GPUBuffer this .

Returns: undefined 

Content timeline steps: 

If this . [[pending_map]] is not null : 

Reject this . [[pending_map]] with an AbortError . 

Set this . [[pending_map]] to null . 

If this . [[mapping]] is null : 

Return. 

For each ArrayBuffer ab in this . [[mapping]] . views : 

Perform DetachArrayBuffer ( ab , "WebGPUBufferMapping"). 

Let bufferUpdate be null . 

If this . [[mapping]] . mode contains WRITE : 

Set bufferUpdate to {
data : this . [[mapping]] . data ,
offset : this . [[mapping]] . range [0]
}. 

Note: When a buffer is mapped without the WRITE mode, then
unmapped, any local modifications done by the application to the mapped ranges
ArrayBuffer are discarded and will not affect the content of later mappings. 

Set this . [[mapping]] to null . 

Issue the subsequent steps on the Device timeline of this . [[device]] . 

Device timeline steps:

If any of the following conditions are unsatisfied, return. 

this is valid to use with this . [[device]] . 

Assert this . [[internal state]] is " unavailable ". 

If bufferUpdate is not null : 

Issue the following steps on the Queue timeline of this . [[device]] . queue : 

Queue timeline steps:

Update the contents of this at offset bufferUpdate . offset 
with the data bufferUpdate . data . 

Set this . [[internal state]] to " available ". 

6. Textures and Texture Views 

6.1. GPUTexture 

A texture is made up of 1d , 2d ,
or 3d arrays of data which can contain multiple values per-element to
represent things like colors. Textures can be read and written in many ways, depending on the
GPUTextureUsage they are created with. For example, textures can be sampled, read, and written
from render and compute pipeline shaders, and they can be written by render pass outputs.
Internally, textures are often stored in GPU memory with a layout optimized for
multidimensional access rather than linear access. 

One texture consists of one or more texture subresources ,
each uniquely identified by a mipmap level and,
for 2d textures only, array layer and aspect . 

A texture subresource is a subresource : each can be used in different
internal usages within a single usage scope . 

Each subresource in a mipmap level is approximately half the size,
in each spatial dimension, of the corresponding resource in the lesser level
(see logical miplevel-specific texture extent ).
The subresource in level 0 has the dimensions of the texture itself.
Smaller levels are typically used to store lower resolution versions of the same image.
GPUSampler and WGSL provide facilities for selecting and interpolating between levels of detail , explicitly or automatically. 

A "2d" texture may be an array of array layer s.
Each subresource in a layer is the same size as the corresponding resources in other layers.
For non-2d textures, all subresources have an array layer index of 0. 

Each subresource has an aspect .
Color textures have just one aspect: color .
Depth-or-stencil format textures may have multiple aspects:
a depth aspect,
a stencil aspect, or both, and may be used in special ways, such as in
depthStencilAttachment and in "depth" bindings. 

A "3d" texture may have multiple slice s, each being the
two-dimensional image at a particular z value in the texture.
Slices are not separate subresources. 

[ Exposed =( Window , Worker ), SecureContext ]
interface GPUTexture {
GPUTextureView createView ( optional GPUTextureViewDescriptor descriptor = {});

undefined destroy ();

readonly attribute GPUIntegerCoordinateOut width ;
readonly attribute GPUIntegerCoordinateOut height ;
readonly attribute GPUIntegerCoordinateOut depthOrArrayLayers ;
readonly attribute GPUIntegerCoordinateOut mipLevelCount ;
readonly attribute GPUSize32Out sampleCount ;
readonly attribute GPUTextureDimension dimension ;
readonly attribute GPUTextureFormat format ;
readonly attribute GPUFlagsConstant usage ;
readonly attribute ( GPUTextureViewDimension or undefined ) textureBindingViewDimension ;
};
GPUTexture includes GPUObjectBase ;

GPUTexture has the following immutable properties : 

width , of type GPUIntegerCoordinateOut , readonly 

The width of this GPUTexture . 

height , of type GPUIntegerCoordinateOut , readonly 

The height of this GPUTexture . 

depthOrArrayLayers , of type GPUIntegerCoordinateOut , readonly 

The depth or layer count of this GPUTexture . 

mipLevelCount , of type GPUIntegerCoordinateOut , readonly 

The number of mip levels of this GPUTexture . 

sampleCount , of type GPUSize32Out , readonly 

The number of sample count of this GPUTexture . 

dimension , of type GPUTextureDimension , readonly 

The dimension of the set of texel for each of this GPUTexture ’s subresources. 

format , of type GPUTextureFormat , readonly 

The format of this GPUTexture . 

usage , of type GPUFlagsConstant , readonly 

The allowed usages for this GPUTexture . 

[[viewFormats]] , of type sequence < GPUTextureFormat >

The set of GPUTextureFormat s that can be used as the
GPUTextureViewDescriptor . format 
when creating views on this GPUTexture . 

textureBindingViewDimension , of type (GPUTextureViewDimension or undefined) , readonly 

On devices without "core-features-and-limits" ,
views created from this texture must have this as their dimension .

On devices with "core-features-and-limits" ,
this is undefined , and there is no such restriction. 

GPUTexture has the following device timeline properties : 

[[destroyed]] , of type boolean , initially false 

If the texture is destroyed, it can no longer be used in any operation,
and its underlying memory can be freed. 

compute render extent (baseSize, mipLevel)

Arguments: 

GPUExtent3D baseSize 

GPUSize32 mipLevel 

Returns: GPUExtent3DDict 

Device timeline steps: 

Let extent be a new GPUExtent3DDict object. 

Set extent . width to max(1, baseSize . width ≫ mipLevel ). 

Set extent . height to max(1, baseSize . height ≫ mipLevel ). 

Set extent . depthOrArrayLayers to 1. 

Return extent . 

The logical miplevel-specific texture extent of a texture is the size of the
texture in texels at a specific miplevel. It is calculated by this procedure: 

Logical miplevel-specific texture extent (descriptor, mipLevel)

Arguments: 

GPUTextureDescriptor descriptor 

GPUSize32 mipLevel 

Returns: GPUExtent3DDict 

Let extent be a new GPUExtent3DDict object. 

If descriptor . dimension is: 

"1d" 

Set extent . width to max(1, descriptor . size . width ≫ mipLevel ). 

Set extent . height to 1. 

Set extent . depthOrArrayLayers to 1. 

"2d" 

Set extent . width to max(1, descriptor . size . width ≫ mipLevel ). 

Set extent . height to max(1, descriptor . size . height ≫ mipLevel ). 

Set extent . depthOrArrayLayers to descriptor . size . depthOrArrayLayers . 

"3d" 

Set extent . width to max(1, descriptor . size . width ≫ mipLevel ). 

Set extent . height to max(1, descriptor . size . height ≫ mipLevel ). 

Set extent . depthOrArrayLayers to max(1, descriptor . size . depthOrArrayLayers ≫ mipLevel ). 

Return extent . 

The physical miplevel-specific texture extent of a texture is the size of the
texture in texels at a specific miplevel that includes the possible extra padding
to form complete texel blocks in the texture . It is calculated by this procedure: 

Physical miplevel-specific texture extent (descriptor, mipLevel)

Arguments: 

GPUTextureDescriptor descriptor 

GPUSize32 mipLevel 

Returns: GPUExtent3DDict 

Let extent be a new GPUExtent3DDict object. 

Let logicalExtent be logical miplevel-specific texture extent ( descriptor , mipLevel ). 

If descriptor . dimension is: 

"1d" 

Set extent . width to logicalExtent . width rounded up to the nearest multiple of descriptor ’s texel block width . 

Set extent . height to 1. 

Set extent . depthOrArrayLayers to 1. 

"2d" 

Set extent . width to logicalExtent . width rounded up to the nearest multiple of descriptor ’s texel block width . 

Set extent . height to logicalExtent . height rounded up to the nearest multiple of descriptor ’s texel block height . 

Set extent . depthOrArrayLayers to logicalExtent . depthOrArrayLayers . 

"3d" 

Set extent . width to logicalExtent . width rounded up to the nearest multiple of descriptor ’s texel block width . 

Set extent . height to logicalExtent . height rounded up to the nearest multiple of descriptor ’s texel block height . 

Set extent . depthOrArrayLayers to logicalExtent . depthOrArrayLayers . 

Return extent . 

6.1.1. GPUTextureDescriptor 

dictionary GPUTextureDescriptor 
: GPUObjectDescriptorBase {
required GPUExtent3D size ;
GPUIntegerCoordinate mipLevelCount = 1;
GPUSize32 sampleCount = 1;
GPUTextureDimension dimension = "2d";
required GPUTextureFormat format ;
required GPUTextureUsageFlags usage ;
sequence < GPUTextureFormat > " href="#dom-gputexturedescriptor-viewformats" id="ref-for-dom-gputexturedescriptor-viewformats"> viewFormats = [];
GPUTextureViewDimension textureBindingViewDimension ;
};

GPUTextureDescriptor has the following members: 

size , of type GPUExtent3D 

The width, height, and depth or layer count of the texture. 

mipLevelCount , of type GPUIntegerCoordinate , defaulting to 1 

The number of mip levels the texture will contain. 

sampleCount , of type GPUSize32 , defaulting to 1 

The sample count of the texture. A sampleCount > 1 indicates
a multisampled texture. 

dimension , of type GPUTextureDimension , defaulting to "2d" 

Whether the texture is one-dimensional, an array of two-dimensional layers, or three-dimensional. 

format , of type GPUTextureFormat 

The format of the texture. 

usage , of type GPUTextureUsageFlags 

The allowed usages for the texture. 

viewFormats , of type sequence< GPUTextureFormat >, defaulting to [] 

Specifies what view format values will be allowed when calling
createView() on this texture (in addition to the texture’s actual
format ). 

NOTE: 

Adding a format to this list may have a significant performance impact, so it is best
to avoid adding formats unnecessarily.

The actual performance impact is highly dependent on the target system; developers must
test various systems to find out the impact on their particular application.
For example, on some systems any texture with a format or
viewFormats entry including
"rgba8unorm-srgb" will perform less optimally than a
"rgba8unorm" texture which does not.
Similar caveats exist for other formats and pairs of formats on other systems. 

Formats in this list must be texture view format compatible with the texture format. 

Two GPUTextureFormat s format and viewFormat are texture view format compatible on a given device if:

format equals viewFormat , or 

format and viewFormat differ only in whether they are srgb formats (have the -srgb suffix) and device . [[features]] contains "core-features-and-limits" . 

textureBindingViewDimension , of type GPUTextureViewDimension 

On devices without "core-features-and-limits" ,
views created from this texture must have this as their dimension .
If not specified, a default is chosen.

On devices with "core-features-and-limits" ,
this is ignored, and there is no such restriction. 

enum GPUTextureDimension {
"1d" ,
"2d" ,
"3d" ,
};

"1d" 

Specifies a texture that has one dimension, width. "1d" textures
cannot have mipmaps, be multisampled, use compressed or depth/stencil formats, or be used as
a render target. 

"2d" 

Specifies a texture that has a width and height, and may have layers. 

"3d" 

Specifies a texture that has a width, height, and depth. "3d" 
textures cannot be multisampled, and their format must support 3d textures
(all plain color formats and some packed/compressed formats ). 

6.1.2. Texture Usages 

typedef [ EnforceRange ] unsigned long GPUTextureUsageFlags ;
[ Exposed =( Window , Worker ), SecureContext ]
namespace GPUTextureUsage {
const GPUFlagsConstant COPY_SRC = 0x01;
const GPUFlagsConstant COPY_DST = 0x02;
const GPUFlagsConstant TEXTURE_BINDING = 0x04;
const GPUFlagsConstant STORAGE_BINDING = 0x08;
const GPUFlagsConstant RENDER_ATTACHMENT = 0x10;
const GPUFlagsConstant TRANSIENT_ATTACHMENT = 0x20;
};

The GPUTextureUsage flags determine how a GPUTexture may be used after its creation: 

COPY_SRC 

The texture can be used as the source of a copy operation. (Examples: as the source 
argument of a copyTextureToTexture() or
copyTextureToBuffer() call.) 

COPY_DST 

The texture can be used as the destination of a copy or write operation. (Examples: as the
destination argument of a copyTextureToTexture() or
copyBufferToTexture() call, or as the target of a
writeTexture() call.) 

TEXTURE_BINDING 

The texture can be bound for use as a sampled texture in a shader (Example: as a bind group
entry for a GPUTextureBindingLayout .) 

STORAGE_BINDING 

The texture can be bound for use as a storage texture in a shader (Example: as a bind group
entry for a GPUStorageTextureBindingLayout .) 

RENDER_ATTACHMENT 

The texture can be used as a color or depth/stencil attachment in a render pass.
(Example: as a GPURenderPassColorAttachment . view or
GPURenderPassDepthStencilAttachment . view .) 

TRANSIENT_ATTACHMENT 

The texture is intended to be temporary (a hint for optimization), as it is only used within
a render pass. 

maximum mipLevel count ( dimension , size )

Arguments: 

GPUTextureDimension dimension 

GPUTextureDimension size 

Calculate the max dimension value m : 

If dimension is: 

"1d" 

Return 1. 

"2d" 

Let m = max( size . width , size . height ). 

"3d" 

Let m = max(max( size . width , size . height ), size . depthOrArrayLayers ). 

Return floor(log 2 ( m )) + 1. 

6.1.3. Texture Creation 

createTexture(descriptor) 

Creates a GPUTexture . 

Called on: GPUDevice this.

Arguments: 

Arguments for the GPUDevice.createTexture(descriptor) method. 

Parameter
Type
Nullable
Optional
Description

descriptor 
GPUTextureDescriptor 
✘ 
✘ 
Description of the GPUTexture to create.

Returns: GPUTexture 

Content timeline steps: 

? validate GPUExtent3D shape ( descriptor . size ). 

? Validate texture format required features of
descriptor . format with this . [[device]] . 

? Validate texture format required features of each element of
descriptor . viewFormats with this . [[device]] . 

Let t be ! create a new WebGPU object ( this , GPUTexture , descriptor ). 

Set t . width to descriptor . size . width . 

Set t . height to descriptor . size . height . 

Set t . depthOrArrayLayers to descriptor . size . depthOrArrayLayers . 

Set t . mipLevelCount to descriptor . mipLevelCount . 

Set t . sampleCount to descriptor . sampleCount . 

Set t . dimension to descriptor . dimension . 

Set t . format to descriptor . format . 

Set t . usage to descriptor . usage . 

If t . [[device]] . [[features]] does not contain "core-features-and-limits" :

If descriptor . textureBindingViewDimension is provided : 

Set t . textureBindingViewDimension to descriptor . textureBindingViewDimension . 

Otherwise, if descriptor . dimension is: 

"1d" 

Set t . textureBindingViewDimension to "1d" . 

"2d" 

If the array layer count of t is 1: 

Set t . textureBindingViewDimension to "2d" . 

Otherwise: 

Set t . textureBindingViewDimension to "2d-array" . 

"3d" 

Set t . textureBindingViewDimension to "3d" . 

Issue the initialization steps on the Device timeline of this . 

Return t . 

Device timeline initialization steps :

If any of the following conditions are unsatisfied
generate a validation error , invalidate t and return. 

validating GPUTextureDescriptor ( this , descriptor ) returns true . 

Set t . [[viewFormats]] to descriptor . viewFormats . 

Create a device allocation for t where each block has an
equivalent texel representation to a block with a bit representation of zero. 

If the allocation fails without side-effects,
generate an out-of-memory error , invalidate t , and return. 

validating GPUTextureDescriptor ( this , descriptor ):

Arguments: 

GPUDevice this 

GPUTextureDescriptor descriptor 

Device timeline steps: 

Let limits be this . [[limits]] . 

Return true if all of the following requirements are met, and false otherwise: 

this must not be lost . 

descriptor . usage must not be 0. 

descriptor . usage must contain only bits present in this ’s allowed texture usages . 

descriptor . size . width ,
descriptor . size . height ,
and descriptor . size . depthOrArrayLayers must be > zero. 

descriptor . mipLevelCount must be > zero. 

descriptor . sampleCount must be either 1 or 4. 

If descriptor . dimension is: 

"1d" 

descriptor . size . width must be ≤
limits . maxTextureDimension1D . 

descriptor . size . height must be 1. 

descriptor . size . depthOrArrayLayers must be 1. 

descriptor . sampleCount must be 1. 

descriptor . format must not be a compressed format or depth-or-stencil format . 

"2d" 

descriptor . size . width must be ≤
limits . maxTextureDimension2D . 

descriptor . size . height must be ≤
limits . maxTextureDimension2D . 

descriptor . size . depthOrArrayLayers must be ≤
limits . maxTextureArrayLayers . 

"3d" 

descriptor . size . width must be ≤
limits . maxTextureDimension3D . 

descriptor . size . height must be ≤
limits . maxTextureDimension3D . 

descriptor . size . depthOrArrayLayers must be ≤
limits . maxTextureDimension3D . 

descriptor . sampleCount must be 1. 

descriptor . format must support "3d" 
textures according to § 26.1 Texture Format Capabilities . 

If this . [[features]] does not contain "core-features-and-limits" :

If descriptor . textureBindingViewDimension is "2d" , this . size . depthOrArrayLayers must be 1. 

if descriptor . textureBindingViewDimension is "cube" , this . size . depthOrArrayLayers must be 6. 

descriptor . textureBindingViewDimension must not be "cube-array" . 

Note: this validation only applies to a user-specified textureBindingViewDimension. If no value is provided, the texture’s textureBindingViewDimension is set as described in createTexture() . That algorithm cannot produce invalid values, so the above validation is not required. 

descriptor . size . width must be multiple of texel block width . 

descriptor . size . height must be multiple of texel block height . 

If descriptor . sampleCount > 1: 

descriptor . mipLevelCount must be 1. 

descriptor . size . depthOrArrayLayers must be 1. 

descriptor . usage must not include the STORAGE_BINDING bit. 

descriptor . usage must include the RENDER_ATTACHMENT bit. 

descriptor . format must support multisampling according to § 26.1 Texture Format Capabilities . 

descriptor . mipLevelCount must be ≤
maximum mipLevel count ( descriptor . dimension , descriptor . size ). 

If descriptor . usage includes the RENDER_ATTACHMENT bit: 

descriptor . format must be a renderable format . 

descriptor . dimension must be either "2d" or "3d" . 

If descriptor . usage includes the STORAGE_BINDING bit: 

descriptor . format must be listed in § 26.1.1 Plain color formats table
with STORAGE_BINDING capability for at least one access mode. 

If descriptor . usage includes the TRANSIENT_ATTACHMENT bit: 

descriptor . usage must be equal to TRANSIENT_ATTACHMENT | RENDER_ATTACHMENT . 

descriptor . dimension must be equal to "2d" . 

descriptor . mipLevelCount must be 1. 

descriptor . size . depthOrArrayLayers must be 1. 

For each viewFormat in descriptor . viewFormats ,
descriptor . format and viewFormat must be
texture view format compatible on device this . 

NOTE: 

Implementations may consider issuing a developer-visible warning if viewFormat is not compatible with any of the
given usage bits, as that viewFormat will be unusable.

Creating a 16x16, RGBA, 2D texture with one array layer and one mip level:

const texture = gpuDevice . createTexture ({ 
size : { width : 16 , height : 16 }, 
format : 'rgba8unorm' , 
usage : GPUTextureUsage . TEXTURE_BINDING , 
}); 

6.1.4. Texture Destruction 

An application that no longer requires a GPUTexture can choose to lose access to it before
garbage collection by calling destroy() . 

Note: This allows the user agent to reclaim the GPU memory associated with the GPUTexture once
all previously submitted operations using it are complete. 

GPUTexture has the following methods: 

destroy() 

Destroys the GPUTexture . 

Called on: GPUTexture this .

Returns: undefined 

Content timeline steps: 

Issue the subsequent steps on the device timeline . 

Device timeline steps:

Set this . [[destroyed]] to true. 

6.2. GPUTextureView 

A GPUTextureView is a view onto some subset of the texture subresources defined by
a particular GPUTexture . 

[ Exposed =( Window , Worker ), SecureContext ]
interface GPUTextureView {
};
GPUTextureView includes GPUObjectBase ;

GPUTextureView has the following immutable properties : 

[[texture]] , readonly

The GPUTexture into which this is a view. 

[[descriptor]] , readonly

The GPUTextureViewDescriptor describing this texture view. 

All optional fields of GPUTextureViewDescriptor are defined. 

[[renderExtent]] , readonly

For renderable views, this is the effective GPUExtent3DDict for rendering. 

Note: this extent depends on the baseMipLevel . 

The set of subresources of a texture view view ,
with [[descriptor]] desc ,
is the subset of the subresources of view . [[texture]] 
for which each subresource s satisfies the following:

The mipmap level of s is ≥
desc . baseMipLevel and <
desc . baseMipLevel +
desc . mipLevelCount . 

The array layer of s is ≥
desc . baseArrayLayer and <
desc . baseArrayLayer +
desc . arrayLayerCount . 

The aspect of s is in the set of aspects of
desc . aspect . 

Two GPUTextureView objects are texture-view-aliasing if and only if
their sets of subresources intersect. 

6.2.1. Texture View Creation 

dictionary GPUTextureViewDescriptor 
: GPUObjectDescriptorBase {
GPUTextureFormat format ;
GPUTextureViewDimension dimension ;
GPUTextureUsageFlags usage = 0;
GPUTextureAspect aspect = "all";
GPUIntegerCoordinate baseMipLevel = 0;
GPUIntegerCoordinate mipLevelCount ;
GPUIntegerCoordinate baseArrayLayer = 0;
GPUIntegerCoordinate arrayLayerCount ;

// Requires "texture-component-swizzle" feature.
DOMString swizzle = "rgba";
};

GPUTextureViewDescriptor has the following members: 

format , of type GPUTextureFormat 

The format of the texture view. Must be either the format of the
texture or one of the viewFormats specified during its creation. 

dimension , of type GPUTextureViewDimension 

The dimension to view the texture as. 

usage , of type GPUTextureUsageFlags , defaulting to 0 

The allowed usage(s) for the texture view. Must be a subset of the
usage flags of the texture. If 0, defaults to the full set of
usage flags of the texture. 

Note: If the view’s format doesn’t support all of the
texture’s usage s, the default will fail,
and the view’s usage must be specified explicitly. 

aspect , of type GPUTextureAspect , defaulting to "all" 

Which aspect(s) of the texture are accessible to the texture view. 

baseMipLevel , of type GPUIntegerCoordinate , defaulting to 0 

The first (most detailed) mipmap level accessible to the texture view. 

mipLevelCount , of type GPUIntegerCoordinate 

How many mipmap levels, starting with baseMipLevel , are accessible to
the texture view. 

baseArrayLayer , of type GPUIntegerCoordinate , defaulting to 0 

The index of the first array layer accessible to the texture view. 

arrayLayerCount , of type GPUIntegerCoordinate 

How many array layers, starting with baseArrayLayer , are accessible
to the texture view. 

swizzle , of type DOMString , defaulting to "rgba" 

A string of length four, with each character mapping to the texture view’s red/green/blue/alpha
channels, respectively. 

When accessed by a shader, the red/green/blue/alpha channels are replaced by the value
corresponding to the component specified in swizzle[0] , swizzle[1] , swizzle[2] , and
swizzle[3] , respectively: 

"r" : Take its value from the red channel of the texture. 

"g" : Take its value from the green channel of the texture. 

"b" : Take its value from the blue channel of the texture. 

"a" : Take its value from the alpha channel of the texture. 

"0" : Force its value to 0. 

"1" : Force its value to 1. 

Requires the "texture-component-swizzle" feature to be enabled. 

enum GPUTextureViewDimension {
"1d" ,
"2d" ,
"2d-array" ,
"cube" ,
"cube-array" ,
"3d" ,
};

"1d" 

The texture is viewed as a 1-dimensional image. 

Corresponding WGSL types: 

texture_1d 

texture_storage_1d 

"2d" 

The texture is viewed as a single 2-dimensional image. 

Corresponding WGSL types: 

texture_2d 

texture_storage_2d 

texture_multisampled_2d 

texture_depth_2d 

texture_depth_multisampled_2d 

"2d-array" 

The texture view is viewed as an array of 2-dimensional images. 

Corresponding WGSL types: 

texture_2d_array 

texture_storage_2d_array 

texture_depth_2d_array 

"cube" 

The texture is viewed as a cubemap. 

The view has 6 array layers, each corresponding to a face of the cube in the order
[+X, -X, +Y, -Y, +Z, -Z] and the following orientations: 

Cubemap faces.
The +U/+V axes indicate the individual faces' texture coordinates,
and thus the texel copy memory layout of each face.

Note: When viewed from the inside, this results in a left-handed coordinate system
where +X is right, +Y is up, and +Z is forward. 

Sampling is done seamlessly across the faces of the cubemap. 

Corresponding WGSL types: 

texture_cube 

texture_depth_cube 

"cube-array" 

The texture is viewed as a packed array of n cubemaps,
each with 6 array layers behaving like one "cube" view,
for 6 n array layers in total. 

Corresponding WGSL types: 

texture_cube_array 

texture_depth_cube_array 

"3d" 

The texture is viewed as a 3-dimensional image. 

Corresponding WGSL types: 

texture_3d 

texture_storage_3d 

Each GPUTextureAspect value corresponds to a set of aspects .
The set of aspects are defined for each value below. 

enum GPUTextureAspect {
"all" ,
"stencil-only" ,
"depth-only" ,
};

"all" 

All available aspects of the texture format will be accessible to the texture view. For
color formats the color aspect will be accessible. For
combined depth-stencil format s both the depth and stencil aspects will be accessible.
Depth-or-stencil format s with a single aspect will only make that aspect accessible. 

The set of aspects is [ color , depth , stencil ]. 

"stencil-only" 

Only the stencil aspect of a depth-or-stencil format format will be accessible to the
texture view. 

The set of aspects is [ stencil ]. 

"depth-only" 

Only the depth aspect of a depth-or-stencil format format will be accessible to the
texture view. 

The set of aspects is [ depth ]. 

createView(descriptor) 

Creates a GPUTextureView . 

NOTE: 

By default createView() will create a view with a dimension that can
represent the entire texture. For example, calling createView() without
specifying a dimension on a "2d" 
texture with more than one layer will create a "2d-array" 
GPUTextureView , even if an arrayLayerCount of 1 is
specified.

For textures created from sources where the layer count is unknown at the
time of development it is recommended that calls to createView() are provided
with an explicit dimension to ensure shader compatibility. 

Called on: GPUTexture this .

Arguments: 

Arguments for the GPUTexture.createView(descriptor) method. 

Parameter
Type
Nullable
Optional
Description

descriptor 
GPUTextureViewDescriptor 
✘ 
✔ 
Description of the GPUTextureView to create.

Returns: view , of type GPUTextureView . 

Content timeline steps: 

? Validate texture format required features of
descriptor . format with this . [[device]] . 

? Validate swizzle string of descriptor . swizzle . 

Let view be ! create a new WebGPU object ( this , GPUTextureView , descriptor ). 

Issue the initialization steps on the Device timeline of this . 

Return view . 

Device timeline initialization steps :

Set descriptor to the result of resolving GPUTextureViewDescriptor defaults 
for this with descriptor . 

If any of the following conditions are unsatisfied
generate a validation error , invalidate view and return. 

this is valid to use with this . [[device]] . 

descriptor . aspect must be present in this . format . 

If the descriptor . aspect is "all" : 

descriptor . format must equal either
this . format or one
of the formats in this . [[viewFormats]] . 

Otherwise: 

descriptor . format must equal the result of resolving GPUTextureAspect (
this . format ,
descriptor . aspect ). 

If descriptor . swizzle is not "rgba" ,
"texture-component-swizzle" must be enabled for this . [[device]] . 

descriptor . usage must be a subset of this . usage . 

If descriptor . usage includes the RENDER_ATTACHMENT bit: 

descriptor . format must be a renderable format . 

If descriptor . usage includes the STORAGE_BINDING bit: 

descriptor . format must be listed in § 26.1.1 Plain color formats table
with STORAGE_BINDING capability for at least one access mode. 

descriptor . mipLevelCount must be > 0. 

descriptor . baseMipLevel +
descriptor . mipLevelCount must be ≤
this . mipLevelCount . 

descriptor . arrayLayerCount must be > 0. 

descriptor . baseArrayLayer +
descriptor . arrayLayerCount must be ≤
the array layer count of this . 

If this . sampleCount > 1,
descriptor . dimension must be "2d" . 

If descriptor . dimension is: 

"1d" 

this . dimension must be "1d" . 

descriptor . arrayLayerCount must be 1 . 

"2d" 

this . dimension must be "2d" . 

descriptor . arrayLayerCount must be 1 . 

"2d-array" 

this . dimension must be "2d" . 

"cube" 

this . dimension must be "2d" . 

descriptor . arrayLayerCount must be 6 . 

this . width must equal this . height . 

"cube-array" 

this . dimension must be "2d" . 

descriptor . arrayLayerCount must be a multiple of 6 . 

this . width must equal this . height . 

[[device]] . [[features]] must contain "core-features-and-limits" .

"3d" 

this . dimension must be "3d" . 

descriptor . arrayLayerCount must be 1 . 

Let view be a new GPUTextureView object. 

Set view . [[texture]] to this . 

Set view . [[descriptor]] to descriptor . 

If descriptor . usage contains RENDER_ATTACHMENT : 

Let renderExtent be compute render extent ([ this . width , this . height , this . depthOrArrayLayers ], descriptor . baseMipLevel ). 

Set view . [[renderExtent]] to renderExtent . 

When resolving GPUTextureViewDescriptor defaults for GPUTextureView 
texture with a GPUTextureViewDescriptor descriptor , run the following device timeline steps:

Let resolved be a copy of descriptor . 

If resolved . format is not provided : 

Let format be the result of resolving GPUTextureAspect (
format ,
descriptor . aspect ). 

If format is null : 

Set resolved . format to texture . format . 

Otherwise: 

Set resolved . format to format . 

If resolved . mipLevelCount is not provided :
set resolved . mipLevelCount to texture . mipLevelCount 
− resolved . baseMipLevel . 

If resolved . dimension is not provided and
texture . dimension is: 

"1d" 

Set resolved . dimension to "1d" . 

"2d" 

If the array layer count of texture is 1: 

Set resolved . dimension to "2d" . 

Otherwise: 

Set resolved . dimension to "2d-array" . 

"3d" 

Set resolved . dimension to "3d" . 

If resolved . arrayLayerCount is not provided and
resolved . dimension is: 

"1d" , "2d" , or
"3d" 

Set resolved . arrayLayerCount to 1 . 

"cube" 

Set resolved . arrayLayerCount to 6 . 

"2d-array" or "cube-array" 

Set resolved . arrayLayerCount to the array layer count of texture 
− resolved . baseArrayLayer . 

If resolved . usage is 0 :
set resolved . usage to texture . usage . 

Return resolved . 

To determine the array layer count of GPUTexture texture , run the
following steps:

If texture . dimension is: 

"1d" or "3d" 

Return 1 . 

"2d" 

Return texture . depthOrArrayLayers . 

To Validate swizzle string of a DOMString swizzle ,
run the following content timeline steps:

If swizzle does not match the [ECMAScript] regexp ^[rgba01]{4}$ : 

Throw a TypeError . 

6.3. Texture Formats 

The name of the format specifies the order of components, bits per component,
and data type for the component. 

r , g , b , a = red, green, blue, alpha 

unorm = unsigned normalized 

snorm = signed normalized 

uint = unsigned int 

sint = signed int 

float = floating point 

If the format has the -srgb suffix, then sRGB conversions from gamma to linear
and vice versa are applied during the reading and writing of color values in the
shader. Compressed texture formats are provided by features . Their naming
should follow the convention here, with the texture name as a prefix. e.g.
etc2-rgba8unorm . 

The texel block is a single addressable element of the textures in pixel-based GPUTextureFormat s,
and a single compressed block of the textures in block-based compressed GPUTextureFormat s. 

The texel block width and texel block height specifies the dimension of one texel block . 

For pixel-based GPUTextureFormat s, the texel block width and texel block height are always 1. 

For block-based compressed GPUTextureFormat s, the texel block width is the number of texels in each row of one texel block ,
and the texel block height is the number of texel rows in one texel block . See § 26.1 Texture Format Capabilities for an exhaustive list
of values for every texture format. 

The texel block copy footprint of an aspect of a GPUTextureFormat is the number of
bytes one texel block occupies during a texel copy , if applicable. 

Note: 
The texel block memory cost of a GPUTextureFormat is the number of
bytes needed to store one texel block . It is not fully defined for all formats.
This value is informative and non-normative. 

enum GPUTextureFormat {
// 8-bit formats
"r8unorm" ,
"r8snorm" ,
"r8uint" ,
"r8sint" ,

// 16-bit formats
"r16unorm" ,
"r16snorm" ,
"r16uint" ,
"r16sint" ,
"r16float" ,
"rg8unorm" ,
"rg8snorm" ,
"rg8uint" ,
"rg8sint" ,

// 32-bit formats
"r32uint" ,
"r32sint" ,
"r32float" ,
"rg16unorm" ,
"rg16snorm" ,
"rg16uint" ,
"rg16sint" ,
"rg16float" ,
"rgba8unorm" ,
"rgba8unorm-srgb" ,
"rgba8snorm" ,
"rgba8uint" ,
"rgba8sint" ,
"bgra8unorm" ,
"bgra8unorm-srgb" ,
// Packed 32-bit formats
"rgb9e5ufloat" ,
"rgb10a2uint" ,
"rgb10a2unorm" ,
"rg11b10ufloat" ,

// 64-bit formats
"rg32uint" ,
"rg32sint" ,
"rg32float" ,
"rgba16unorm" ,
"rgba16snorm" ,
"rgba16uint" ,
"rgba16sint" ,
"rgba16float" ,

// 128-bit formats
"rgba32uint" ,
"rgba32sint" ,
"rgba32float" ,

// Depth/stencil formats
"stencil8" ,
"depth16unorm" ,
"depth24plus" ,
"depth24plus-stencil8" ,
"depth32float" ,

// "depth32float-stencil8" feature
"depth32float-stencil8" ,

// BC compressed formats usable if "texture-compression-bc" is both
// supported by the device/user agent and enabled in requestDevice.
"bc1-rgba-unorm" ,
"bc1-rgba-unorm-srgb" ,
"bc2-rgba-unorm" ,
"bc2-rgba-unorm-srgb" ,
"bc3-rgba-unorm" ,
"bc3-rgba-unorm-srgb" ,
"bc4-r-unorm" ,
"bc4-r-snorm" ,
"bc5-rg-unorm" ,
"bc5-rg-snorm" ,
"bc6h-rgb-ufloat" ,
"bc6h-rgb-float" ,
"bc7-rgba-unorm" ,
"bc7-rgba-unorm-srgb" ,

// ETC2 compressed formats usable if "texture-compression-etc2" is both
// supported by the device/user agent and enabled in requestDevice.
"etc2-rgb8unorm" ,
"etc2-rgb8unorm-srgb" ,
"etc2-rgb8a1unorm" ,
"etc2-rgb8a1unorm-srgb" ,
"etc2-rgba8unorm" ,
"etc2-rgba8unorm-srgb" ,
"eac-r11unorm" ,
"eac-r11snorm" ,
"eac-rg11unorm" ,
"eac-rg11snorm" ,

// ASTC compressed formats usable if "texture-compression-astc" is both
// supported by the device/user agent and enabled in requestDevice.
"astc-4x4-unorm" ,
"astc-4x4-unorm-srgb" ,
"astc-5x4-unorm" ,
"astc-5x4-unorm-srgb" ,
"astc-5x5-unorm" ,
"astc-5x5-unorm-srgb" ,
"astc-6x5-unorm" ,
"astc-6x5-unorm-srgb" ,
"astc-6x6-unorm" ,
"astc-6x6-unorm-srgb" ,
"astc-8x5-unorm" ,
"astc-8x5-unorm-srgb" ,
"astc-8x6-unorm" ,
"astc-8x6-unorm-srgb" ,
"astc-8x8-unorm" ,
"astc-8x8-unorm-srgb" ,
"astc-10x5-unorm" ,
"astc-10x5-unorm-srgb" ,
"astc-10x6-unorm" ,
"astc-10x6-unorm-srgb" ,
"astc-10x8-unorm" ,
"astc-10x8-unorm-srgb" ,
"astc-10x10-unorm" ,
"astc-10x10-unorm-srgb" ,
"astc-12x10-unorm" ,
"astc-12x10-unorm-srgb" ,
"astc-12x12-unorm" ,
"astc-12x12-unorm-srgb" ,
};

The depth component of the "depth24plus" and "depth24plus-stencil8" 
formats may be implemented as either a 24-bit depth value or a "depth32float" value.

The stencil8 format may be implemented as
either a real "stencil8", or "depth24stencil8", where the depth aspect is
hidden and inaccessible. 

NOTE: 

While the precision of depth32float channels is strictly higher than the precision of
24-bit depth channels for all values in the representable range (0.0 to 1.0),
note that the set of representable values is not an exact superset.

For 24-bit depth , 1 ULP has a constant value of 1 / (2 24 − 1). 

For depth32float, 1 ULP has a variable value no greater than 1 / (2 24 ). 

A format is renderable if it is either a color renderable format , or a depth-or-stencil format .
If a format is listed in § 26.1.1 Plain color formats with RENDER_ATTACHMENT capability, it is a
color renderable format. Any other format is not a color renderable format.
All depth-or-stencil formats are renderable. 

A renderable format is also blendable 
if it can be used with render pipeline blending.
See § 26.1 Texture Format Capabilities . 

A format is filterable if it supports the
GPUTextureSampleType "float" 
(not just "unfilterable-float" );
that is, it can be used with "filtering" GPUSampler s.
See § 26.1 Texture Format Capabilities . 

resolving GPUTextureAspect (format, aspect)

Arguments: 

GPUTextureFormat format 

GPUTextureAspect aspect 

Returns: GPUTextureFormat or null 

If aspect is: 

"all" 

Return format . 

"depth-only" 
"stencil-only" 

If format is a depth-stencil-format:
Return the aspect-specific format of format according to § 26.1.2 Depth-stencil formats or null if
the aspect is not present in format . 

Return null . 

Use of some texture formats require a feature to be enabled on the GPUDevice . Because new
formats can be added to the specification, those enum values might not be known by the implementation.
In order to normalize behavior across implementations, attempting to use a format that requires a
feature will throw an exception if the associated feature is not enabled on the device. This makes
the behavior the same as when the format is unknown to the implementation. 

See § 26.1 Texture Format Capabilities for information about which GPUTextureFormat s require features. 

To Validate texture format required features of a GPUTextureFormat format 

with logical device device , run the following content timeline steps:

If format requires a feature and device . [[features]] does not contain 
the feature: 

Throw a TypeError . 

6.4. GPUExternalTexture 

A GPUExternalTexture is a sampleable 2D texture wrapping an external video frame.
It is an immutable snapshot; its contents cannot change over time, either from inside WebGPU
(it is only sampleable) or from outside WebGPU (e.g. due to video frame advancement). 

GPUExternalTexture s can be bound into bind groups via the
externalTexture bind group layout entry member.
Note that member uses several binding slots, as defined there. 

NOTE: 

GPUExternalTexture can be implemented without creating a copy of the imported source,
but this depends implementation-defined factors.
Ownership of the underlying representation may either be exclusive or shared with other
owners (such as a video decoder), but this is not visible to the application.

The underlying representation of an external texture is unobservable
(except for precise sampling behavior), but typically may include: 

Up to three 2D planes of data (e.g. RGBA, Y+UV, Y+U+V). 

Metadata for converting coordinates before reading from those planes (crop and rotation). 

Metadata for converting values into the specified output color space (matrices, gammas, 3D LUT). 

The configuration used internally by an implementation may be inconsistent across time,
systems, user agents, media sources, or even frames within a single video source.
In order to account for many possible representations,
the binding conservatively uses the following, for each external texture: 

three sampled texture bindings (for up to 3 planes), 

one sampled texture binding for a 3D LUT, 

one sampler binding to sample the 3D LUT, and 

one uniform buffer binding for metadata. 

[ Exposed =( Window , Worker ), SecureContext ]
interface GPUExternalTexture {
};
GPUExternalTexture includes GPUObjectBase ;

GPUExternalTexture has the following immutable properties : 

[[descriptor]] , of type GPUExternalTextureDescriptor , readonly

The descriptor with which the texture was created. 

GPUExternalTexture has the following immutable properties : 

[[expired]] , of type boolean , initially false 

Indicates whether the object has expired (can no longer be used). 

Note: 
Unlike [[destroyed]] slots, which are similar, this can change from true back to false . 

6.4.1. Importing External Textures 

An external texture is created from an external video object
using importExternalTexture() . 

An external texture created from an HTMLVideoElement expires (is destroyed) automatically in a
task after it is imported, instead of manually or upon garbage collection like other resources.
When an external texture expires, its [[expired]] slot changes to true . 

An external texture created from a VideoFrame expires (is destroyed) when, and only when,
the source VideoFrame is closed ,
either explicitly by close() , or by other means. 

Note: As noted in decode() , authors should call
close() on output VideoFrame s to avoid decoder stalls.
If an imported VideoFrame is dropped without being closed, the imported
GPUExternalTexture object will keep it alive until it is also dropped.
The VideoFrame cannot be garbage collected until both objects are dropped.
Garbage collection is unpredictable, so this may still stall the video decoder. 

Once the GPUExternalTexture expires, importExternalTexture() must be called again.
However, the user agent may un-expire and return the same GPUExternalTexture again, instead of
creating a new one. This will commonly happen unless the execution of the application is scheduled
to match the video’s frame rate (e.g. using requestVideoFrameCallback() ).
If the same object is returned again, it will compare equal, and GPUBindGroup s,
GPURenderBundle s, etc. referencing the previous object can still be used. 

dictionary GPUExternalTextureDescriptor 
: GPUObjectDescriptorBase {
required ( HTMLVideoElement or VideoFrame ) source ;
PredefinedColorSpace colorSpace = "srgb";
};

GPUExternalTextureDescriptor dictionaries have the following members: 

source , of type (HTMLVideoElement or VideoFrame) 

The video source to import the external texture from. Source size is determined as described
by the external source dimensions table. 

colorSpace , of type PredefinedColorSpace , defaulting to "srgb" 

The color space the image contents of source will be
converted into when reading. 

importExternalTexture(descriptor) 

Creates a GPUExternalTexture wrapping the provided image source. 

Called on: GPUDevice this .

Arguments: 

Arguments for the GPUDevice.importExternalTexture(descriptor) method. 

Parameter
Type
Nullable
Optional
Description

descriptor 
GPUExternalTextureDescriptor 
✘ 
✘ 
Provides the external image source object (and any creation options).

Returns: GPUExternalTexture 

Content timeline steps: 

Let source be descriptor . source . 

If the current image contents of source are the same as the most recent
importExternalTexture() call with the same descriptor 
(ignoring label ),
and the user agent chooses to reuse it: 

Let previousResult be the GPUExternalTexture returned previously. 

Set previousResult . [[expired]] to false ,
renewing ownership of the underlying resource. 

Let result be previousResult . 

Note: 
This allows the application to detect duplicate imports and avoid re-creating
dependent objects (such as GPUBindGroup s).
Implementations still need to be able to handle a single frame being wrapped by
multiple GPUExternalTexture , since import metadata like
colorSpace can change even for the same frame. 

Otherwise: 

If source is not origin-clean ,
throw a SecurityError and return. 

Let usability be ? check the usability of the image argument ( source ). 

If usability is not good : 

Generate a validation error . 

Return an invalidated GPUExternalTexture . 

Let data be the result of converting the current image contents of source into
the color space descriptor . colorSpace 
with unpremultiplied alpha. 

This may result in values outside of the range [0, 1].
If clamping is desired, it may be performed after sampling. 

Note: This is described like a copy, but may be implemented as a reference to
read-only underlying data plus appropriate metadata to perform conversion later. 

Let result be a new GPUExternalTexture object wrapping data . 

If source is an HTMLVideoElement ,
queue an automatic expiry task with device this and the following steps: 

Set result . [[expired]] to true ,
releasing ownership of the underlying resource. 

Note: 
An HTMLVideoElement should be imported in the same task that samples the texture
(which should generally be scheduled using requestVideoFrameCallback or
requestAnimationFrame() depending on the application).
Otherwise, a texture could get destroyed by these steps before the
application is finished using it. 

If source is a VideoFrame , then when source is
closed , run the following steps: 

Set result . [[expired]] to true . 

Set result . label to descriptor . label . 

Return result . 

Rendering using an video element external texture at the page animation frame rate:

const videoElement = document . createElement ( 'video' ); 
// ... set up videoElement, wait for it to be ready... 

function frame () { 
requestAnimationFrame ( frame ); 

// Always re-import the video on every animation frame, because the 
// import is likely to have expired. 
// The browser may cache and reuse a past frame, and if it does it 
// may return the same GPUExternalTexture object again. 
// In this case, old bind groups are still valid. 
const externalTexture = gpuDevice . importExternalTexture ({ 
source : videoElement
}); 

// ... render using externalTexture... 
} 
requestAnimationFrame ( frame ); 

Rendering using an video element external texture at the video’s frame rate, if
requestVideoFrameCallback is available:

const videoElement = document . createElement ( 'video' ); 
// ... set up videoElement... 

function frame () { 
videoElement . requestVideoFrameCallback ( frame ); 

// Always re-import, because we know the video frame has advanced 
const externalTexture = gpuDevice . importExternalTexture ({ 
source : videoElement
}); 

// ... render using externalTexture... 
} 
videoElement . requestVideoFrameCallback ( frame ); 

6.5. Sampling External Texture Bindings 

The externalTexture binding point allows binding GPUExternalTexture 
objects (from dynamic image sources like videos). It also supports GPUTexture and GPUTextureView . 

Note: 
When a GPUTexture or a GPUTextureView is bound to an externalTexture 
binding, it is like a GPUExternalTexture with a single RGBA plane and no crop, rotation, or color
conversion. 

External textures are represented in WGSL with texture_external and may be read using
textureLoad and textureSampleBaseClampToEdge . 

The sampler provided to textureSampleBaseClampToEdge is used to sample the underlying textures. 

When the binding resource type is a GPUExternalTexture , the result is in the color space set
by colorSpace .
It is implementation-dependent whether, for any given external texture, the sampler (and filtering)
is applied before or after conversion from underlying values into the specified color space. 

Note: 
If the internal representation is an RGBA plane, sampling behaves as on a regular 2D texture.
If there are several underlying planes (e.g. Y+UV), the sampler is used to sample each
underlying texture separately, prior to conversion from YUV to the specified color space. 

7. Samplers 

7.1. GPUSampler 

A GPUSampler encodes transformations and filtering information that can
be used in a shader to interpret texture resource data. 

GPUSampler s are created via createSampler() . 

[ Exposed =( Window , Worker ), SecureContext ]
interface GPUSampler {
};
GPUSampler includes GPUObjectBase ;

GPUSampler has the following immutable properties : 

[[descriptor]] , of type GPUSamplerDescriptor , readonly

The GPUSamplerDescriptor with which the GPUSampler was created. 

[[isComparison]] , of type boolean , readonly

Whether the GPUSampler is used as a comparison sampler. 

[[isFiltering]] , of type boolean , readonly

Whether the GPUSampler weights multiple samples of a texture. 

7.1.1. GPUSamplerDescriptor 

A GPUSamplerDescriptor specifies the options to use to create a GPUSampler . 

dictionary GPUSamplerDescriptor 
: GPUObjectDescriptorBase {
GPUAddressMode addressModeU = "clamp-to-edge";
GPUAddressMode addressModeV = "clamp-to-edge";
GPUAddressMode addressModeW = "clamp-to-edge";
GPUFilterMode magFilter = "nearest";
GPUFilterMode minFilter = "nearest";
GPUMipmapFilterMode mipmapFilter = "nearest";
float lodMinClamp = 0;
float lodMaxClamp = 32;
GPUCompareFunction compare ;
[ Clamp ] unsigned short maxAnisotropy = 1;
};

addressModeU , of type GPUAddressMode , defaulting to "clamp-to-edge" 
addressModeV , of type GPUAddressMode , defaulting to "clamp-to-edge" 
addressModeW , of type GPUAddressMode , defaulting to "clamp-to-edge" 

Specifies the address modes for the texture width, height, and depth
coordinates, respectively. 

magFilter , of type GPUFilterMode , defaulting to "nearest" 

Specifies the sampling behavior when the sampled area is smaller than or equal to one
texel. 

minFilter , of type GPUFilterMode , defaulting to "nearest" 

Specifies the sampling behavior when the sampled area is larger than one texel. 

mipmapFilter , of type GPUMipmapFilterMode , defaulting to "nearest" 

Specifies behavior for sampling between mipmap levels. 

lodMinClamp , of type float , defaulting to 0 
lodMaxClamp , of type float , defaulting to 32 

Specifies the minimum and maximum levels of detail , respectively, used internally when
sampling a texture. 

compare , of type GPUCompareFunction 

When provided the sampler will be a comparison sampler with the specified
GPUCompareFunction . 

Note: Comparison samplers may use filtering, but the sampling results will be
implementation-dependent and may differ from the normal filtering rules. 

maxAnisotropy , of type unsigned short , defaulting to 1 

Specifies the maximum anisotropy value clamp used by the sampler. Anisotropic filtering is
enabled when maxAnisotropy is > 1 and the implementation supports it. 

Anisotropic filtering improves the image quality of textures sampled at oblique viewing
angles. Higher maxAnisotropy values indicate the maximum ratio of
anisotropy supported when filtering. 

NOTE: 

Most implementations support maxAnisotropy values in range
between 1 and 16, inclusive. The used value of maxAnisotropy 
will be clamped to the maximum value that the platform supports.

The precise filtering behavior is implementation-dependent. 

Level of detail (LOD) describes which mip level(s) are selected when sampling a
texture. It may be specified explicitly through shader methods like textureSampleLevel or implicitly determined from
the texture coordinate derivatives. 

Note: See Scale Factor Operation, LOD Operation and Image Level Selection in
the Vulkan 1.3 spec for an example of how implicit LODs may be calculated. 

GPUAddressMode describes the behavior of the sampler if the sampled texels extend beyond the
bounds of the sampled texture. 

enum GPUAddressMode {
"clamp-to-edge" ,
"repeat" ,
"mirror-repeat" ,
};

"clamp-to-edge" 

Texture coordinates are clamped between 0.0 and 1.0, inclusive. 

"repeat" 

Texture coordinates wrap to the other side of the texture. 

"mirror-repeat" 

Texture coordinates wrap to the other side of the texture, but the texture is flipped
when the integer part of the coordinate is odd. 

GPUFilterMode and GPUMipmapFilterMode describe the behavior of the sampler if the sampled
area does not cover exactly one texel. 

Note: See Texel Filtering in the Vulkan 1.3 spec for an example of how
samplers may determine which texels are sampled from for the various filtering modes. 

enum GPUFilterMode {
"nearest" ,
"linear" ,
};

enum GPUMipmapFilterMode {
"nearest" ,
"linear" ,
};

"nearest" 

Return the value of the texel nearest to the texture coordinates. 

"linear" 

Select two texels in each dimension and return a linear interpolation between their values. 

GPUCompareFunction specifies the behavior of a comparison sampler. If a comparison sampler is
used in a shader, the depth_ref is compared to the fetched texel value, and the result of this
comparison test is generated ( 1.0f for pass, or 0.0f for fail). 

After comparison, if texture filtering is enabled, the filtering step occurs, so that comparison
results are mixed together resulting in values in the range [0, 1] . Filtering should behave
as usual, however it may be computed with lower precision or not mix results at all. 

enum GPUCompareFunction {
"never" ,
"less" ,
"equal" ,
"less-equal" ,
"greater" ,
"not-equal" ,
"greater-equal" ,
"always" ,
};

"never" 

Comparison tests never pass. 

"less" 

A provided value passes the comparison test if it is less than the sampled value. 

"equal" 

A provided value passes the comparison test if it is equal to the sampled value. 

"less-equal" 

A provided value passes the comparison test if it is less than or equal to the sampled value. 

"greater" 

A provided value passes the comparison test if it is greater than the sampled value. 

"not-equal" 

A provided value passes the comparison test if it is not equal to the sampled value. 

"greater-equal" 

A provided value passes the comparison test if it is greater than or equal to the sampled value. 

"always" 

Comparison tests always pass. 

7.1.2. Sampler Creation 

createSampler(descriptor) 

Creates a GPUSampler . 

Called on: GPUDevice this.

Arguments: 

Arguments for the GPUDevice.createSampler(descriptor) method. 

Parameter
Type
Nullable
Optional
Description

descriptor 
GPUSamplerDescriptor 
✘ 
✔ 
Description of the GPUSampler to create.

Returns: GPUSampler 

Content timeline steps: 

Let s be ! create a new WebGPU object ( this , GPUSampler , descriptor ). 

Issue the initialization steps on the Device timeline of this . 

Return s . 

Device timeline initialization steps :

If any of the following conditions are unsatisfied
generate a validation error , invalidate s and return. 

this must not be lost . 

descriptor . lodMinClamp ≥ 0. 

descriptor . lodMaxClamp ≥
descriptor . lodMinClamp . 

descriptor . maxAnisotropy ≥ 1. 

Note: Most implementations support maxAnisotropy 
values in range between 1 and 16, inclusive. The provided
maxAnisotropy value will be clamped to the
maximum value that the platform supports. 

If descriptor . maxAnisotropy > 1: 

descriptor . magFilter ,
descriptor . minFilter ,
and descriptor . mipmapFilter must be
"linear" . 

Set s . [[descriptor]] to descriptor . 

Set s . [[isComparison]] to false if the compare attribute
of s . [[descriptor]] is null or undefined. Otherwise, set it to true . 

Set s . [[isFiltering]] to false if none of minFilter ,
magFilter , or mipmapFilter has the value of
"linear" . Otherwise, set it to true . 

Creating a GPUSampler that does trilinear filtering and repeats texture coordinates:

const sampler = gpuDevice . createSampler ({ 
addressModeU : 'repeat' , 
addressModeV : 'repeat' , 
magFilter : 'linear' , 
minFilter : 'linear' , 
mipmapFilter : 'linear' , 
}); 

8. Resource Binding 

8.1. GPUBindGroupLayout 

A GPUBindGroupLayout defines the interface between a set of resources bound in a GPUBindGroup and their accessibility in shader stages. 

[ Exposed =( Window , Worker ), SecureContext ]
interface GPUBindGroupLayout {
};
GPUBindGroupLayout includes GPUObjectBase ;

GPUBindGroupLayout has the following immutable properties : 

[[descriptor]] , of type GPUBindGroupLayoutDescriptor , readonly

8.1.1. Bind Group Layout Creation 

A GPUBindGroupLayout is created via GPUDevice.createBindGroupLayout() . 

dictionary GPUBindGroupLayoutDescriptor 
: GPUObjectDescriptorBase {
required sequence < GPUBindGroupLayoutEntry > " href="#dom-gpubindgrouplayoutdescriptor-entries" id="ref-for-dom-gpubindgrouplayoutdescriptor-entries"> entries ;
};

GPUBindGroupLayoutDescriptor dictionaries have the following members: 

entries , of type sequence< GPUBindGroupLayoutEntry > 

A list of entries describing the shader resource bindings for a bind group. 

A GPUBindGroupLayoutEntry describes a single shader resource binding to be included in a GPUBindGroupLayout . 

dictionary GPUBindGroupLayoutEntry {
required GPUIndex32 binding ;
required GPUShaderStageFlags visibility ;

GPUBufferBindingLayout buffer ;
GPUSamplerBindingLayout sampler ;
GPUTextureBindingLayout texture ;
GPUStorageTextureBindingLayout storageTexture ;
GPUExternalTextureBindingLayout externalTexture ;
};

GPUBindGroupLayoutEntry dictionaries have the following members: 

binding , of type GPUIndex32 

A unique identifier for a resource binding within the GPUBindGroupLayout , corresponding
to a GPUBindGroupEntry.binding and a @binding 
attribute in the GPUShaderModule . 

visibility , of type GPUShaderStageFlags 

A bitset of the members of GPUShaderStage .
Each set bit indicates that a GPUBindGroupLayoutEntry ’s resource
will be accessible from the associated shader stage. 

buffer , of type GPUBufferBindingLayout 
sampler , of type GPUSamplerBindingLayout 
texture , of type GPUTextureBindingLayout 
storageTexture , of type GPUStorageTextureBindingLayout 
externalTexture , of type GPUExternalTextureBindingLayout 

Exactly one of these members must be set, indicating the binding type.
The contents of the member specify options specific to that type. 

The corresponding resource in createBindGroup() requires
the corresponding binding resource type for this binding. 

typedef [ EnforceRange ] unsigned long GPUShaderStageFlags ;
[ Exposed =( Window , Worker ), SecureContext ]
namespace GPUShaderStage {
const GPUFlagsConstant VERTEX = 0x1;
const GPUFlagsConstant FRAGMENT = 0x2;
const GPUFlagsConstant COMPUTE = 0x4;
};

GPUShaderStage contains the following flags, which describe which shader stages a
corresponding GPUBindGroupEntry for this GPUBindGroupLayoutEntry will be visible to: 

VERTEX 

The bind group entry will be accessible to vertex shaders. 

FRAGMENT 

The bind group entry will be accessible to fragment shaders. 

COMPUTE 

The bind group entry will be accessible to compute shaders. 

The binding member of a GPUBindGroupLayoutEntry is determined by which member of the
GPUBindGroupLayoutEntry is defined:
buffer , sampler ,
texture , storageTexture , or
externalTexture .
Only one may be defined for any given GPUBindGroupLayoutEntry .
Each member has an associated GPUBindingResource 
type and each binding type has an associated internal usage , given by this table: 

Binding member 

Resource type 

Binding type 

Binding usage 

buffer 

GPUBufferBinding 
(or GPUBuffer as shorthand )

"uniform" 

constant 

"storage" 

storage 

"read-only-storage" 

storage-read 

sampler 

GPUSampler 

"filtering" 

constant 

"non-filtering" 

"comparison" 

texture 

GPUTextureView 
(or GPUTexture as shorthand )

"float" 

constant 

"unfilterable-float" 

"depth" 

"sint" 

"uint" 

storageTexture 

GPUTextureView 
(or GPUTexture as shorthand )

"write-only" 

storage 

"read-write" 

"read-only" 

storage-read 

externalTexture 

GPUExternalTexture 
or GPUTextureView 
(or GPUTexture as shorthand )

constant 

The list of GPUBindGroupLayoutEntry values entries 
exceeds the binding slot limits of supported limits limits 
if the number of slots used toward a limit exceeds the supported value in limits .
Each entry may use multiple slots toward multiple limits.

Device timeline steps: 

For each entry in entries , if: 

entry . buffer ?. type 
is "uniform" and
entry . buffer ?. hasDynamicOffset is true 

Consider 1 maxDynamicUniformBuffersPerPipelineLayout slot to be used. 

entry . buffer ?. type 
is "storage" and
entry . buffer ?. hasDynamicOffset is true 

Consider 1 maxDynamicStorageBuffersPerPipelineLayout slot to be used. 

For each shader stage stage in
« VERTEX , FRAGMENT , COMPUTE »: 

For each entry in entries for which
entry . visibility contains stage , if: 

entry . buffer ?. type 
is "uniform" 

Consider 1 maxUniformBuffersPerShaderStage slot to be used. 

entry . buffer ?. type 
is "storage" or "read-only-storage" 

If stage is: 

VERTEX 

Consider 1 maxStorageBuffersInVertexStage slot to be used. 

FRAGMENT 

Consider 1 maxStorageBuffersInFragmentStage slot to be used. 

COMPUTE 

Consider 1 maxStorageBuffersPerShaderStage slot to be used. 

entry . sampler is provided 

Consider 1 maxSamplersPerShaderStage slot to be used. 

entry . texture is provided 

Consider 1 maxSampledTexturesPerShaderStage slot to be used. 

entry . storageTexture is provided 

If stage is: 

VERTEX 

Consider 1 maxStorageTexturesInVertexStage slot to be used. 

FRAGMENT 

Consider 1 maxStorageTexturesInFragmentStage slot to be used. 

COMPUTE 

Consider 1 maxStorageTexturesPerShaderStage slot to be used. 

entry . externalTexture is provided 

Consider
4 maxSampledTexturesPerShaderStage slot,
1 maxSamplersPerShaderStage slot, and
1 maxUniformBuffersPerShaderStage slot
to be used. 

Note: See GPUExternalTexture for an explanation of this behavior. 

enum GPUBufferBindingType {
"uniform" ,
"storage" ,
"read-only-storage" ,
};

dictionary GPUBufferBindingLayout {
GPUBufferBindingType type = "uniform";
boolean hasDynamicOffset = false ;
GPUSize64 minBindingSize = 0;
};

GPUBufferBindingLayout dictionaries have the following members: 

type , of type GPUBufferBindingType , defaulting to "uniform" 

Indicates the type required for buffers bound to this binding. 

hasDynamicOffset , of type boolean , defaulting to false 

Indicates whether this binding requires a dynamic offset. 

minBindingSize , of type GPUSize64 , defaulting to 0 

Indicates the minimum size of a buffer binding used with this bind point. 

Bindings are always validated against this size in createBindGroup() . 

If this is not 0 , pipeline creation additionally validates 
that this value ≥ the minimum buffer binding size of the variable. 

If this is 0 , it is ignored by pipeline creation, and instead draw/dispatch commands
validate that each binding in the GPUBindGroup 
satisfies the minimum buffer binding size of the variable. 

Note: 
Similar execution-time validation is theoretically possible for other
binding-related fields specified for early validation, like
sampleType and format ,
which currently can only be validated in pipeline creation.
However, such execution-time validation could be costly or unnecessarily complex, so it is
available only for minBindingSize which is expected to have the
most ergonomic impact. 

enum GPUSamplerBindingType {
"filtering" ,
"non-filtering" ,
"comparison" ,
};

dictionary GPUSamplerBindingLayout {
GPUSamplerBindingType type = "filtering";
};

GPUSamplerBindingLayout dictionaries have the following members: 

type , of type GPUSamplerBindingType , defaulting to "filtering" 

Indicates the required type of a sampler bound to this binding. 

enum GPUTextureSampleType {
"float" ,
"unfilterable-float" ,
"depth" ,
"sint" ,
"uint" ,
};

dictionary GPUTextureBindingLayout {
GPUTextureSampleType sampleType = "float";
GPUTextureViewDimension viewDimension = "2d";
boolean multisampled = false ;
};

GPUTextureBindingLayout dictionaries have the following members: 

sampleType , of type GPUTextureSampleType , defaulting to "float" 

Indicates the type required for texture views bound to this binding. 

viewDimension , of type GPUTextureViewDimension , defaulting to "2d" 

Indicates the required dimension for texture views bound to
this binding. 

multisampled , of type boolean , defaulting to false 

Indicates whether or not texture views bound to this binding must be multisampled. 

enum GPUStorageTextureAccess {
"write-only" ,
"read-only" ,
"read-write" ,
};

dictionary GPUStorageTextureBindingLayout {
GPUStorageTextureAccess access = "write-only";
required GPUTextureFormat format ;
GPUTextureViewDimension viewDimension = "2d";
};

GPUStorageTextureBindingLayout dictionaries have the following members: 

access , of type GPUStorageTextureAccess , defaulting to "write-only" 

The access mode for this binding, indicating readability and writability. 

format , of type GPUTextureFormat 

The required format of texture views bound to this binding. 

viewDimension , of type GPUTextureViewDimension , defaulting to "2d" 

Indicates the required dimension for texture views bound to
this binding. 

dictionary GPUExternalTextureBindingLayout {
};

A GPUBindGroupLayout object has the following device timeline properties : 

[[entryMap]] , of type ordered map < GPUSize32 , GPUBindGroupLayoutEntry >, readonly

The map of binding indices pointing to the GPUBindGroupLayoutEntry s,
which this GPUBindGroupLayout describes. 

[[dynamicOffsetCount]] , of type GPUSize32 , readonly

The number of buffer bindings with dynamic offsets in this GPUBindGroupLayout . 

[[exclusivePipeline]] , of type GPUPipelineBase ?, readonly

The pipeline that created this GPUBindGroupLayout , if it was created as part of a
default pipeline layout . If not null , GPUBindGroup s
created with this GPUBindGroupLayout can only be used with the specified
GPUPipelineBase . 

createBindGroupLayout(descriptor) 

Creates a GPUBindGroupLayout . 

Called on: GPUDevice this .

Arguments: 

Arguments for the GPUDevice.createBindGroupLayout(descriptor) method. 

Parameter
Type
Nullable
Optional
Description

descriptor 
GPUBindGroupLayoutDescriptor 
✘ 
✘ 
Description of the GPUBindGroupLayout to create.

Returns: GPUBindGroupLayout 

Content timeline steps: 

For each GPUBindGroupLayoutEntry entry in descriptor . entries : 

If entry . storageTexture is provided : 

? Validate texture format required features for
entry . storageTexture . format 
with this . [[device]] . 

Let layout be ! create a new WebGPU object ( this , GPUBindGroupLayout , descriptor ). 

Issue the initialization steps on the Device timeline of this . 

Return layout . 

Device timeline initialization steps :

If any of the following conditions are unsatisfied
generate a validation error , invalidate layout and return. 

this must not be lost . 

Let limits be this . [[device]] . [[limits]] . 

The binding of each entry in descriptor is unique. 

The binding of each entry in descriptor must be <
limits . maxBindingsPerBindGroup . 

descriptor . entries must not
exceed the binding slot limits of limits . 

For each GPUBindGroupLayoutEntry entry in descriptor . entries : 

Exactly one of
entry . buffer ,
entry . sampler ,
entry . texture ,
entry . storageTexture , and
entry . externalTexture is provided . 

entry . visibility contains only bits defined in GPUShaderStage . 

If entry . visibility includes
VERTEX : 

If entry . buffer is provided ,
entry . buffer . type 
must be "uniform" or "read-only-storage" . 

If entry . storageTexture is provided ,
entry . storageTexture . access 
must be "read-only" . 

If entry . texture ?. multisampled is true : 

entry . texture . viewDimension is
"2d" . 

entry . texture . sampleType is not
"float" . 

If entry . storageTexture is provided : 

entry . storageTexture . viewDimension is not
"cube" or "cube-array" . 

entry . storageTexture . format must be a format
which can support storage usage for the given
entry . storageTexture . access 
according to the § 26.1.1 Plain color formats table. 

Set layout . [[descriptor]] to descriptor . 

Set layout . [[dynamicOffsetCount]] to the number of
entries in descriptor where buffer is provided and
buffer . hasDynamicOffset is true . 

Set layout . [[exclusivePipeline]] to null . 

For each GPUBindGroupLayoutEntry entry in
descriptor . entries : 

Insert entry into layout . [[entryMap]] 
with the key of entry . binding . 

8.1.2. Compatibility 

Two GPUBindGroupLayout objects a and b are considered group-equivalent 
if and only if all of the following conditions are satisfied:

a . [[exclusivePipeline]] == b . [[exclusivePipeline]] . 

for any binding number binding , one of the following conditions is satisfied: 

it’s missing from both a . [[entryMap]] and b . [[entryMap]] . 

a . [[entryMap]] [ binding ] == b . [[entryMap]] [ binding ] 

If bind groups layouts are group-equivalent they can be interchangeably used in all contents. 

8.2. GPUBindGroup 

A GPUBindGroup defines a set of resources to be bound together in a group
and how the resources are used in shader stages. 

[ Exposed =( Window , Worker ), SecureContext ]
interface GPUBindGroup {
};
GPUBindGroup includes GPUObjectBase ;

GPUBindGroup has the following device timeline properties : 

[[layout]] , of type GPUBindGroupLayout , readonly

The GPUBindGroupLayout associated with this GPUBindGroup . 

[[entries]] , of type sequence < GPUBindGroupEntry >, readonly

The set of GPUBindGroupEntry s this GPUBindGroup describes. 

[[usedResources]] , of type usage scope , readonly

The set of buffer and texture subresource s used by this bind group,
associated with lists of the internal usage flags. 

The bound buffer ranges of a GPUBindGroup bindGroup ,
given list <GPUBufferDynamicOffset> dynamicOffsets , are computed as follows:

Let result be a new set <( GPUBindGroupLayoutEntry , GPUBufferBinding )>. 

Let dynamicOffsetIndex be 0. 

For each GPUBindGroupEntry bindGroupEntry in bindGroup . [[entries]] ,
sorted by bindGroupEntry . binding : 

Let bindGroupLayoutEntry be
bindGroup . [[layout]] . [[entryMap]] [ bindGroupEntry . binding ]. 

If bindGroupLayoutEntry . buffer is not
provided , continue . 

Let bound be get as buffer binding ( bindGroupEntry . resource ). 

If bindGroupLayoutEntry . buffer . hasDynamicOffset : 

Increment bound . offset by
dynamicOffsets [ dynamicOffsetIndex ]. 

Increment dynamicOffsetIndex by 1. 

Append ( bindGroupLayoutEntry , bound ) to result . 

Return result . 

8.2.1. Bind Group Creation 

A GPUBindGroup is created via GPUDevice.createBindGroup() . 

dictionary GPUBindGroupDescriptor 
: GPUObjectDescriptorBase {
required GPUBindGroupLayout layout ;
required sequence < GPUBindGroupEntry > " href="#dom-gpubindgroupdescriptor-entries" id="ref-for-dom-gpubindgroupdescriptor-entries"> entries ;
};

GPUBindGroupDescriptor dictionaries have the following members: 

layout , of type GPUBindGroupLayout 

The GPUBindGroupLayout the entries of this bind group will conform to. 

entries , of type sequence< GPUBindGroupEntry > 

A list of entries describing the resources to expose to the shader for each binding
described by the layout . 

typedef ( GPUSampler or 
GPUTexture or 
GPUTextureView or 
GPUBuffer or 
GPUBufferBinding or 
GPUExternalTexture ) GPUBindingResource ;

dictionary GPUBindGroupEntry {
required GPUIndex32 binding ;
required GPUBindingResource resource ;
};

A GPUBindGroupEntry describes a single resource to be bound in a GPUBindGroup , and has the
following members: 

binding , of type GPUIndex32 

A unique identifier for a resource binding within the GPUBindGroup , corresponding to a
GPUBindGroupLayoutEntry.binding and a @binding 
attribute in the GPUShaderModule . 

resource , of type GPUBindingResource 

The resource to bind, which may be a GPUSampler , GPUTexture , GPUTextureView ,
GPUBuffer , GPUBufferBinding , or GPUExternalTexture . 

GPUBindGroupEntry has the following device timeline properties : 

[[prevalidatedSize]] , of type boolean 

Whether or not this binding entry had its buffer size validated at time of creation. 

dictionary GPUBufferBinding {
required GPUBuffer buffer ;
GPUSize64 offset = 0;
GPUSize64 size ;
};

A GPUBufferBinding describes a buffer and optional range to bind as a resource, and has the
following members: 

buffer , of type GPUBuffer 

The GPUBuffer to bind. 

offset , of type GPUSize64 , defaulting to 0 

The offset, in bytes, from the beginning of buffer to the
beginning of the range exposed to the shader by the buffer binding. 

size , of type GPUSize64 

The size, in bytes, of the buffer binding.
If not provided , specifies the range starting at
offset and ending at the end of buffer . 

createBindGroup(descriptor) 

Creates a GPUBindGroup . 

Called on: GPUDevice this .

Arguments: 

Arguments for the GPUDevice.createBindGroup(descriptor) method. 

Parameter
Type
Nullable
Optional
Description

descriptor 
GPUBindGroupDescriptor 
✘ 
✘ 
Description of the GPUBindGroup to create.

Returns: GPUBindGroup 

Content timeline steps: 

Let bindGroup be ! create a new WebGPU object ( this , GPUBindGroup , descriptor ). 

Issue the initialization steps on the Device timeline of this . 

Return bindGroup . 

Device timeline initialization steps :

Let limits be this . [[device]] . [[limits]] . 

If any of the following conditions are unsatisfied
generate a validation error , invalidate bindGroup and return. 

descriptor . layout is valid to use with this . 

The number of entries of
descriptor . layout is exactly equal to
the number of descriptor . entries . 

For each GPUBindGroupEntry bindingDescriptor in
descriptor . entries : 

Let resource be bindingDescriptor . resource . 

There is exactly one GPUBindGroupLayoutEntry layoutBinding 
in descriptor . layout . entries 
such that layoutBinding . binding equals to
bindingDescriptor . binding . 

If the defined binding member for layoutBinding is: 

sampler 

resource is a GPUSampler . 

resource is valid to use with this . 

If layoutBinding . sampler . type is: 

"filtering" 

resource . [[isComparison]] is false . 

"non-filtering" 

resource . [[isFiltering]] is false .
resource . [[isComparison]] is false . 

"comparison" 

resource . [[isComparison]] is true . 

texture 

resource is either a GPUTexture or a GPUTextureView . 

resource is valid to use with this . 

Let textureView be get as texture view ( resource ). 

Let texture be textureView . [[texture]] . 

layoutBinding . texture . viewDimension 
is equal to textureView ’s dimension . 

layoutBinding . texture . sampleType 
is compatible with
textureView ’s format . 

textureView . [[descriptor]] . usage 
includes TEXTURE_BINDING . 

If layoutBinding . texture . multisampled 
is true , texture ’s sampleCount 
> 1 , Otherwise texture ’s sampleCount is 1 . 

If texture . textureBindingViewDimension is not undefined :

Assert this . [[device]] . [[features]] does not contain "core-features-and-limits" . 

texture . textureBindingViewDimension must be equal to textureView . dimension . 

storageTexture 

resource is either a GPUTexture or a GPUTextureView . 

resource is valid to use with this . 

Let storageTextureView be get as texture view ( resource ). 

Let texture be storageTextureView . [[texture]] . 

layoutBinding . storageTexture . viewDimension 
is equal to storageTextureView ’s dimension . 

layoutBinding . storageTexture . format 
is equal to storageTextureView . [[descriptor]] . format . 

storageTextureView . [[descriptor]] . usage 
includes STORAGE_BINDING . 

storageTextureView . [[descriptor]] . mipLevelCount must be 1. 

storageTextureView . [[descriptor]] . swizzle must be "rgba" . 

buffer 

resource is either a GPUBuffer or a GPUBufferBinding . 

Let bufferBinding be get as buffer binding ( resource ). 

bufferBinding . buffer is valid to use with this . 

The bound part designated by bufferBinding . offset and
bufferBinding . size resides inside the buffer and has non-zero size. 

effective buffer binding size ( bufferBinding ) ≥
layoutBinding . buffer . minBindingSize . 

If layoutBinding . buffer . type is 

"uniform" 

bufferBinding . buffer . usage 
includes UNIFORM . 

effective buffer binding size ( bufferBinding ) ≤
limits . maxUniformBufferBindingSize . 

bufferBinding . offset is a multiple of
limits . minUniformBufferOffsetAlignment . 

"storage" or
"read-only-storage" 

bufferBinding . buffer . usage 
includes STORAGE . 

effective buffer binding size ( bufferBinding ) ≤
limits . maxStorageBufferBindingSize . 

effective buffer binding size ( bufferBinding ) is a multiple of 4. 

bufferBinding . offset is a multiple of
limits . minStorageBufferOffsetAlignment . 

externalTexture 

resource is either a GPUExternalTexture , a GPUTexture , or a GPUTextureView . 

resource is valid to use with this . 

If resource is a: 

GPUTexture or GPUTextureView 

Let view be get as texture view ( resource ). 

view . [[descriptor]] . usage 
must include TEXTURE_BINDING . 

view . [[descriptor]] . dimension 
must be "2d" . 

view . [[descriptor]] . mipLevelCount 
must be 1. 

view . [[descriptor]] . format 
must be "rgba8unorm" ,
"bgra8unorm" , or
"rgba16float" . 

view . [[texture]] . sampleCount 
must be 1. 

If this . [[device]] . [[features]] does not contain "core-features-and-limits" :

For each GPUBindGroupEntry bindGroupEntry in descriptor . entries : 

If bindGroupEntry . resource is a GPUTextureView : 

Let textureView be bindGroupEntry . resource . 

Let descriptor be textureView . [[descriptor]] . 

descriptor . baseArrayLayer must be 0 . 

descriptor . arrayLayerCount must be equal to textureView . [[texture]] . depthOrArrayLayers . 

Let bindGroup . [[layout]] =
descriptor . layout . 

Let bindGroup . [[entries]] =
descriptor . entries . 

Let bindGroup . [[usedResources]] = {}. 

For each GPUBindGroupEntry bindingDescriptor in
descriptor . entries : 

Let internalUsage be the binding usage for layoutBinding . 

Each subresource seen by resource is added to [[usedResources]] as internalUsage . 

Let bindingDescriptor . [[prevalidatedSize]] be false if the defined
binding member for layoutBinding is buffer 
and layoutBinding . buffer . minBindingSize 
is 0 , and true otherwise. 

get as texture view ( resource )

Arguments: 

GPUBindingResource resource 

Returns: GPUTextureView 

Assert resource is either a GPUTexture or a GPUTextureView . 

If resource is a: 

GPUTexture 

Return resource . createView() . 

GPUTextureView 

Return resource . 

get as buffer binding ( resource )

Arguments: 

GPUBindingResource resource 

Returns: GPUBufferBinding 

Assert resource is either a GPUBuffer or a GPUBufferBinding . 

If resource is a: 

GPUBuffer 

Let bufferBinding a new GPUBufferBinding . 

Set bufferBinding . buffer to resource . 

Return bufferBinding . 

GPUBufferBinding 

Return resource . 

effective buffer binding size ( binding )

Arguments: 

GPUBufferBinding binding 

Returns: GPUSize64 

If binding . size is not provided : 

Return max(0, binding . buffer . size - binding . offset ); 

Return binding . size . 

Two GPUBufferBinding objects a and b are considered buffer-binding-aliasing if and only if all of the following are true:

a . buffer == b . buffer 

The range formed by a . offset and a . size intersects
the range formed by b . offset and b . size ,
where if a size is unspecified ,
the range goes to the end of the buffer. 

Note: When doing this calculation, any dynamic offsets have already been applied to the ranges. 

8.3. GPUPipelineLayout 

A GPUPipelineLayout defines the mapping between resources of all GPUBindGroup objects set up during command encoding in setBindGroup() , and the shaders of the pipeline set by GPURenderCommandsMixin.setPipeline or GPUComputePassEncoder.setPipeline . 

The full binding address of a resource can be defined as a trio of: 

shader stage mask, to which the resource is visible 

bind group index 

binding number 

The components of this address can also be seen as the binding space of a pipeline. A GPUBindGroup (with the corresponding GPUBindGroupLayout ) covers that space for a fixed bind group index. The contained bindings need to be a superset of the resources used by the shader at this bind group index. 

[ Exposed =( Window , Worker ), SecureContext ]
interface GPUPipelineLayout {
};
GPUPipelineLayout includes GPUObjectBase ;

GPUPipelineLayout has the following device timeline properties : 

[[bindGroupLayouts]] , of type list < GPUBindGroupLayout >, readonly

The GPUBindGroupLayout objects provided at creation in GPUPipelineLayoutDescriptor.bindGroupLayouts . 

Note: using the same GPUPipelineLayout for many GPURenderPipeline or GPUComputePipeline pipelines guarantees that the user agent doesn’t need to rebind any resources internally when there is a switch between these pipelines. 

GPUComputePipeline object X was created with GPUPipelineLayout.bindGroupLayouts A, B, C. GPUComputePipeline object Y was created with GPUPipelineLayout.bindGroupLayouts A, D, C. Supposing the command encoding sequence has two dispatches:

setBindGroup (0, ...) 

setBindGroup (1, ...) 

setBindGroup (2, ...) 

setPipeline (X) 

dispatchWorkgroups () 

setBindGroup (1, ...) 

setPipeline (Y) 

dispatchWorkgroups () 

In this scenario, the user agent would have to re-bind the group slot 2 for the second dispatch, even though neither the GPUBindGroupLayout at index 2 of GPUPipelineLayout.bindGroupLayouts , or the GPUBindGroup at slot 2, change. 

Note: the expected usage of the GPUPipelineLayout is placing the most common and the least frequently changing bind groups at the "bottom" of the layout, meaning lower bind group slot numbers, like 0 or 1. The more frequently a bind group needs to change between draw calls, the higher its index should be. This general guideline allows the user agent to minimize state changes between draw calls, and consequently lower the CPU overhead. 

8.3.1. Pipeline Layout Creation 

A GPUPipelineLayout is created via GPUDevice.createPipelineLayout() . 

dictionary GPUPipelineLayoutDescriptor 
: GPUObjectDescriptorBase {
required sequence < GPUBindGroupLayout ?> " href="#dom-gpupipelinelayoutdescriptor-bindgrouplayouts" id="ref-for-dom-gpupipelinelayoutdescriptor-bindgrouplayouts②"> bindGroupLayouts ;
};

GPUPipelineLayoutDescriptor dictionaries define all the GPUBindGroupLayout s used by a
pipeline, and have the following members: 

bindGroupLayouts , of type sequence<GPUBindGroupLayout?> 

A list of optional GPUBindGroupLayout s the pipeline will use. Each element corresponds
to a @group attribute in the GPUShaderModule , with the N th element corresponding
with @group(N) . 

createPipelineLayout(descriptor) 

Creates a GPUPipelineLayout . 

Called on: GPUDevice this .

Arguments: 

Arguments for the GPUDevice.createPipelineLayout(descriptor) method. 

Parameter
Type
Nullable
Optional
Description

descriptor 
GPUPipelineLayoutDescriptor 
✘ 
✘ 
Description of the GPUPipelineLayout to create.

Returns: GPUPipelineLayout 

Content timeline steps: 

Let pl be ! create a new WebGPU object ( this , GPUPipelineLayout , descriptor ). 

Issue the initialization steps on the Device timeline of this . 

Return pl . 

Device timeline initialization steps :

Let limits be this . [[device]] . [[limits]] . 

Let bindGroupLayouts be a list of null GPUBindGroupLayout s with size 
equal to limits . maxBindGroups . 

For each bindGroupLayout at index i in descriptor . bindGroupLayouts : 

If bindGroupLayout is not null and
bindGroupLayout . [[descriptor]] . entries 
is not empty : 

Set bindGroupLayouts [ i ] to bindGroupLayout . 

Let allEntries be the result of concatenating
bgl . [[descriptor]] . entries 
for all non- null bgl in bindGroupLayouts . 

If any of the following conditions are unsatisfied
generate a validation error , invalidate pl and return. 

Every non- null GPUBindGroupLayout in bindGroupLayouts 
must be valid to use with this and have a [[exclusivePipeline]] 
of null . 

The size of descriptor . bindGroupLayouts 
must be ≤ limits . maxBindGroups . 

allEntries must not exceed the binding slot limits of limits . 

Set the pl . [[bindGroupLayouts]] to bindGroupLayouts . 

Note: two GPUPipelineLayout objects are considered equivalent for any usage
if their internal [[bindGroupLayouts]] sequences contain
GPUBindGroupLayout objects that are group-equivalent . 

8.4. Example 

Create a GPUBindGroupLayout that describes a binding with a uniform buffer, a texture, and a sampler.
Then create a GPUBindGroup and a GPUPipelineLayout using the GPUBindGroupLayout .

const bindGroupLayout = gpuDevice . createBindGroupLayout ({ 
entries : [{ 
binding : 0 , 
visibility : GPUShaderStage . VERTEX | GPUShaderStage . FRAGMENT , 
buffer : {} 
}, { 
binding : 1 , 
visibility : GPUShaderStage . FRAGMENT , 
texture : {} 
}, { 
binding : 2 , 
visibility : GPUShaderStage . FRAGMENT , 
sampler : {} 
}] 
}); 

const bindGroup = gpuDevice . createBindGroup ({ 
layout : bindGroupLayout , 
entries : [{ 
binding : 0 , 
resource : { buffer : buffer }, 
}, { 
binding : 1 , 
resource : texture
}, { 
binding : 2 , 
resource : sampler
}] 
}); 

const pipelineLayout = gpuDevice . createPipelineLayout ({ 
bindGroupLayouts : [ bindGroupLayout ] 
}); 

9. Shader Modules 

9.1. GPUShaderModule 

[ Exposed =( Window , Worker ), SecureContext ]
interface GPUShaderModule {
Promise < GPUCompilationInfo > getCompilationInfo ();
};
GPUShaderModule includes GPUObjectBase ;

GPUShaderModule is a reference to an internal shader module object. 

9.1.1. Shader Module Creation 

dictionary GPUShaderModuleDescriptor 
: GPUObjectDescriptorBase {
required USVString code ;
sequence < GPUShaderModuleCompilationHint > " href="#dom-gpushadermoduledescriptor-compilationhints" id="ref-for-dom-gpushadermoduledescriptor-compilationhints"> compilationHints = [];
};

code , of type USVString 

The WGSL source code for the shader
module. 

compilationHints , of type sequence< GPUShaderModuleCompilationHint >, defaulting to [] 

A list of GPUShaderModuleCompilationHint s. 

Any hint provided by an application should contain information about one entry point of
a pipeline that will eventually be created from the entry point. 

Implementations should use any information present in the GPUShaderModuleCompilationHint 
to perform as much compilation as is possible within createShaderModule() . 

Aside from type-checking, these hints are not validated in any way. 

NOTE: 

Supplying information in compilationHints does not have any
observable effect, other than performance. It may be detrimental to performance to
provide hints for pipelines that never end up being created.

Because a single shader module can hold multiple entry points, and multiple pipelines
can be created from a single shader module, it can be more performant for an
implementation to do as much compilation as possible once in
createShaderModule() rather than multiple times in the multiple calls to
createComputePipeline() or createRenderPipeline() . 

Hints are only applied to the entry points they explicitly name.
Unlike GPUProgrammableStage.entryPoint ,
there is no default, even if only one entry point is present in the module. 

Note: 
Hints are not validated in an observable way, but user agents may surface identifiable
errors (like unknown entry point names or incompatible pipeline layouts) to developers,
for example in the browser developer console. 

createShaderModule(descriptor) 

Creates a GPUShaderModule . 

Called on: GPUDevice this.

Arguments: 

Arguments for the GPUDevice.createShaderModule(descriptor) method. 

Parameter
Type
Nullable
Optional
Description

descriptor 
GPUShaderModuleDescriptor 
✘ 
✘ 
Description of the GPUShaderModule to create.

Returns: GPUShaderModule 

Content timeline steps: 

Let sm be ! create a new WebGPU object ( this , GPUShaderModule , descriptor ). 

Issue the initialization steps on the Device timeline of this . 

Return sm . 

Device timeline initialization steps :

Let error be any error that results from shader module creation with the
WGSL source descriptor . code , or null if no
errors occured. 

If any of the following requirements are unmet,
generate a validation error , invalidate sm , and return. 

this must not be lost . 

error must not be a shader-creation program error . 

For each enable extension in descriptor . code ,
the corresponding GPUFeatureName must be enabled
(see the Feature Index ). 

Note: Uncategorized errors cannot arise from shader module creation.
Implementations which detect such errors during shader module creation
must behave as if the shader module is valid, and defer surfacing the
error until pipeline creation. 

NOTE: 

User agents should not include detailed compiler error messages or shader text in
the message text of validation errors arising here:
these details are accessible via getCompilationInfo() .
User agents should surface human-readable, formatted error details to
developers for easier debugging (for example as a warning in the browser developer
console, expandable to show full shader source).

As shader compilation errors should be rare in production applications, user agents
could choose to surface them to developers regardless of error handling ( GPU error scopes or
uncapturederror event handlers), e.g. as an expandable warning.
If not, they should provide and document another way for developers to access
human-readable error details, for example by adding a checkbox to show errors
unconditionally, or by showing human-readable details when logging a
GPUCompilationInfo object to the console. 

Create a GPUShaderModule from WGSL code:

// A simple vertex and fragment shader pair that will fill the viewport with red. 
const shaderSource = ` 
var<private> pos : array<vec2<f32>, 3> = array<vec2<f32>, 3>( 
vec2(-1.0, -1.0), vec2(-1.0, 3.0), vec2(3.0, -1.0)); 

@vertex 
fn vertexMain(@builtin(vertex_index) vertexIndex : u32) -> @builtin(position) vec4<f32> { 
return vec4(pos[vertexIndex], 1.0, 1.0); 
} 

@fragment 
fn fragmentMain() -> @location(0) vec4<f32> { 
return vec4(1.0, 0.0, 0.0, 1.0); 
} 
` ; 

const shaderModule = gpuDevice . createShaderModule ({ 
code : shaderSource , 
}); 

9.1.1.1. Shader Module Compilation Hints 

Shader module compilation hints are optional, additional information indicating how a given
GPUShaderModule entry point is intended to be used in the future. For some implementations this
information may aid in compiling the shader module earlier, potentially increasing performance. 

dictionary GPUShaderModuleCompilationHint {
required USVString entryPoint ;
( GPUPipelineLayout or GPUAutoLayoutMode ) layout ;
};

layout , of type (GPUPipelineLayout or GPUAutoLayoutMode) 

A GPUPipelineLayout that the GPUShaderModule may be used with in a future
createComputePipeline() or createRenderPipeline() call.
If set to "auto" the layout will be the default pipeline layout 
for the entry point associated with this hint will be used. 

NOTE: 

If possible, authors should be supplying the same information to
createShaderModule() and createComputePipeline() /
createRenderPipeline() .

If an application is unable to provide hint information at the time of calling
createShaderModule() , it should usually not delay calling
createShaderModule() , but instead just omit the unknown information from
the compilationHints sequence or the individual members of
GPUShaderModuleCompilationHint . Omitting this information
may cause compilation to be deferred to createComputePipeline() /
createRenderPipeline() . 

If an author is not confident that the hint information passed to createShaderModule() 
will match the information later passed to createComputePipeline() /
createRenderPipeline() with that same module, they should avoid passing that
information to createShaderModule() , as passing mismatched information to
createShaderModule() may cause unnecessary compilations to occur. 

9.1.2. Shader Module Compilation Information 

enum GPUCompilationMessageType {
"error" ,
"warning" ,
"info" ,
};

[ Exposed =( Window , Worker ), Serializable , SecureContext ]
interface GPUCompilationMessage {
readonly attribute DOMString message ;
readonly attribute GPUCompilationMessageType type ;
readonly attribute unsigned long long lineNum ;
readonly attribute unsigned long long linePos ;
readonly attribute unsigned long long offset ;
readonly attribute unsigned long long length ;
};

[ Exposed =( Window , Worker ), Serializable , SecureContext ]
interface GPUCompilationInfo {
readonly attribute FrozenArray < GPUCompilationMessage > " id="dom-gpucompilationinfo-messages"> messages ;
};

A GPUCompilationMessage is an informational, warning, or error message generated by the
GPUShaderModule compiler. The messages are intended to be human readable to help developers
diagnose issues with their shader code . Each message may correspond to
a single point or range of the shader source, or may be unassociated with any specific part of the code. 

GPUCompilationMessage has the following attributes: 

message , of type DOMString , readonly 

The human-readable, localizable text for this compilation message. 

Note: The message should follow the best practices for language and direction information . This includes making use of any future standards which may
emerge regarding the reporting of string language and direction metadata. 

Editorial note: 
At the time of this writing, no language/direction recommendation is available that provides
compatibility and consistency with legacy APIs, but when there is, adopt it formally.

type , of type GPUCompilationMessageType , readonly 

The severity level of the message. 

If the type is "error" , it
corresponds to a shader-creation error . 

lineNum , of type unsigned long long , readonly 

The line number in the shader code the
message corresponds to. Value is one-based, such that a lineNum of
1 indicates the first line of the shader code . Lines are
delimited by line breaks . 

If the message corresponds to a substring this points to
the line on which the substring begins. Must be 0 if the message 
does not correspond to any specific point in the shader code . 

linePos , of type unsigned long long , readonly 

The offset, in UTF-16 code units, from the beginning of line lineNum 
of the shader code to the point or beginning of the substring
that the message corresponds to. Value is one-based, such that a
linePos of 1 indicates the first code unit of the line. 

If message corresponds to a substring this points to the
first UTF-16 code unit of the substring. Must be 0 if the message 
does not correspond to any specific point in the shader code . 

offset , of type unsigned long long , readonly 

The offset from the beginning of the shader code in UTF-16
code units to the point or beginning of the substring that message 
corresponds to. Must reference the same position as lineNum and
linePos . Must be 0 if the message 
does not correspond to any specific point in the shader code . 

length , of type unsigned long long , readonly 

The number of UTF-16 code units in the substring that message 
corresponds to. If the message does not correspond with a substring then
length must be 0. 

Note: GPUCompilationMessage . lineNum and
GPUCompilationMessage . linePos are one-based since the most common use
for them is expected to be printing human readable messages that can be correlated with the line and
column numbers shown in many text editors. 

Note: GPUCompilationMessage . offset and
GPUCompilationMessage . length are appropriate to pass to
substr() in order to retrieve the substring of the shader code the
message corresponds to. 

getCompilationInfo() 

Returns any messages generated during the GPUShaderModule ’s compilation. 

The locations, order, and contents of messages are implementation-defined .
In particular, messages aren’t necessarily ordered by lineNum . 

Called on: GPUShaderModule this

Returns: Promise < GPUCompilationInfo > 

Content timeline steps: 

Let contentTimeline be the current Content timeline . 

Let promise be a new promise . 

Issue the synchronization steps on the Device timeline of this . 

Return promise . 

Device timeline synchronization steps :

Let event occur upon the (successful or unsuccessful) completion of
shader module creation for this . 

Listen for timeline event event 
on this . [[device]] , handled by
the subsequent steps on contentTimeline . 

Content timeline steps:

Let info be a new GPUCompilationInfo . 

Let messages be a list of any errors, warnings, or informational messages
generated during shader module creation for this , or the empty list
[] if the device was lost. 

For each message in messages : 

Let m be a new GPUCompilationMessage . 

Set m . message to be the text of message . 

If message is a shader-creation error :

Set m . type to
"error" 

If message is a warning:

Set m . type to
"warning" 

Otherwise:

Set m . type to
"info" 

If message is associated with a specific substring or position
within the shader code :

Set m . lineNum to the one-based number
of the first line that the message refers to. 

Set m . linePos to the one-based number
of the first UTF-16 code units on m . lineNum 
that the message refers to, or 1 if the message refers to
the entire line. 

Set m . offset to the number of UTF-16
code units from the beginning of the shader to beginning of the
substring or position that message refers to. 

Set m . length the length of the
substring in UTF-16 code units that message refers to, or 0
if message refers to a position 

Otherwise:

Set m . lineNum to 0 . 

Set m . linePos to 0 . 

Set m . offset to 0 . 

Set m . length to 0 . 

Append m to info . messages . 

Resolve promise with info . 

10. Pipelines 

A pipeline , be it GPUComputePipeline or GPURenderPipeline ,
represents the complete function done by a combination of the GPU hardware, the driver,
and the user agent, that process the input data in the shape of bindings and vertex buffers,
and produces some output, like the colors in the output render targets. 

Structurally, the pipeline consists of a sequence of programmable stages (shaders)
and fixed-function states, such as the blending modes. 

Note: Internally, depending on the target platform,
the driver may convert some of the fixed-function states into shader code,
and link it together with the shaders provided by the user.
This linking is one of the reason the object is created as a whole. 

This combination state is created as a single object
(a GPUComputePipeline or GPURenderPipeline )
and switched using one command
( GPUComputePassEncoder . setPipeline() or
GPURenderCommandsMixin . setPipeline() respectively). 

There are two ways to create pipelines: 

immediate pipeline creation 

createComputePipeline() and createRenderPipeline() 
return a pipeline object which can be used immediately in a pass encoder. 

When this fails, the pipeline object will be invalid and the call will generate either a
validation error or an internal error . 

Note: 
A handle object is returned immediately, but actual pipeline creation is not synchronous.
If pipeline creation takes a long time, this can incur a stall in the
device timeline at some point between the creation call and execution of the
submit() in which it is first used.
The point is unspecified, but most likely to be one of: at creation, at the first usage of the
pipeline in setPipeline() , at the corresponding finish() of that GPUCommandEncoder or
GPURenderBundleEncoder , or at submit() of that GPUCommandBuffer . 

async pipeline creation 

createComputePipelineAsync() and createRenderPipelineAsync() 
return a Promise which resolves to a pipeline object when creation of the pipeline has
completed. 

When this fails, the Promise rejects with a GPUPipelineError . 

GPUPipelineError describes a pipeline creation failure. 

[ Exposed =( Window , Worker ), SecureContext , Serializable ]
interface GPUPipelineError : DOMException {
constructor ( optional DOMString message = "", GPUPipelineErrorInit options );
readonly attribute GPUPipelineErrorReason reason ;
};

dictionary GPUPipelineErrorInit {
required GPUPipelineErrorReason reason ;
};

enum GPUPipelineErrorReason {
"validation" ,
"internal" ,
};

GPUPipelineError constructor: 

constructor() 

Arguments: 

Arguments for the GPUPipelineError.constructor() method. 

Parameter
Type
Nullable
Optional
Description

message 
DOMString 
✘ 
✔ 
Error message of the base DOMException .

options 
GPUPipelineErrorInit 
✘ 
✘ 
Options specific to GPUPipelineError .

Content timeline steps: 

Set this . name to "GPUPipelineError" . 

Set this . message to message . 

Set this . reason to options . reason . 

GPUPipelineError has the following attributes: 

reason , of type GPUPipelineErrorReason , readonly 

A read-only slot-backed attribute exposing the type of error encountered in pipeline creation
as a GPUPipelineErrorReason : 

"validation" : A validation error . 

"internal" : An internal error . 

GPUPipelineError objects are serializable objects . 

Their serialization steps , given value and serialized , are:

Run the DOMException serialization steps given value and serialized . 

Their deserialization steps , given value and serialized , are:

Run the DOMException deserialization steps given value and serialized . 

10.1. Base pipelines 

enum GPUAutoLayoutMode {
"auto" ,
};

dictionary GPUPipelineDescriptorBase 
: GPUObjectDescriptorBase {
required ( GPUPipelineLayout or GPUAutoLayoutMode ) layout ;
};

layout , of type (GPUPipelineLayout or GPUAutoLayoutMode) 

The GPUPipelineLayout for this pipeline, or "auto" to generate
the pipeline layout automatically. 

Note: If "auto" is used the pipeline cannot share GPUBindGroup s
with any other pipelines. 

interface mixin GPUPipelineBase {
[ NewObject ] GPUBindGroupLayout getBindGroupLayout ( unsigned long index );
};

GPUPipelineBase has the following device timeline properties : 

[[layout]] , of type GPUPipelineLayout 

The definition of the layout of resources which can be used with this . 

GPUPipelineBase has the following methods: 

getBindGroupLayout(index) 

Gets a GPUBindGroupLayout that is compatible with the GPUPipelineBase ’s
GPUBindGroupLayout at index . 

Called on: GPUPipelineBase this 

Arguments: 

Arguments for the GPUPipelineBase.getBindGroupLayout(index) method. 

Parameter
Type
Nullable
Optional
Description

index 
unsigned long 
✘ 
✘ 
Index into the pipeline layout’s [[bindGroupLayouts]] 
sequence.

Returns: GPUBindGroupLayout 

Content timeline steps: 

Let layout be a new GPUBindGroupLayout object. 

Issue the initialization steps on the Device timeline of this . 

Return layout . 

Device timeline initialization steps :

Let limits be this . [[device]] . [[limits]] . 

If any of the following conditions are unsatisfied
generate a validation error , invalidate layout and return. 

this must be valid . 

index < limits . maxBindGroups . 

Initialize layout so it is a copy of
this . [[layout]] . [[bindGroupLayouts]] [ index ]. 

Note: GPUBindGroupLayout is only ever used by-value, not by-reference,
so this is equivalent to returning the same internal object with a new WebGPU interface .
A new GPUBindGroupLayout WebGPU interface is returned each time to avoid a round-trip
between the Content timeline and the Device timeline . 

10.1.1. Default pipeline layout 

A GPUPipelineBase object that was created with a layout set to
"auto" has a default layout created and used instead. 

Note: Default layouts are provided as a convenience for simple pipelines, but use of explicit layouts
is recommended in most cases. Bind groups created from default layouts cannot be used with other
pipelines, and the structure of the default layout may change when altering shaders, causing
unexpected bind group creation errors. 

To create a default pipeline layout for GPUPipelineBase pipeline ,
run the following device timeline steps: 

Let groupCount be 0. 

Let groupDescs be a sequence of device . [[limits]] . maxBindGroups 
new GPUBindGroupLayoutDescriptor objects. 

For each groupDesc in groupDescs : 

Set groupDesc . entries to an empty sequence . 

For each GPUProgrammableStage stageDesc in the descriptor used to create pipeline : 

Let shaderStage be the GPUShaderStageFlags for the shader stage
at which stageDesc is used in pipeline . 

Let entryPoint be get the entry point ( shaderStage , stageDesc ). Assert entryPoint is not null . 

For each resource resource statically used by entryPoint : 

Let group be resource ’s "group" decoration. 

Let binding be resource ’s "binding" decoration. 

Let entry be a new GPUBindGroupLayoutEntry . 

Set entry . binding to binding . 

Set entry . visibility to shaderStage . 

If resource is for a sampler binding: 

Let samplerLayout be a new GPUSamplerBindingLayout . 

Set entry . sampler to samplerLayout . 

If resource is for a comparison sampler binding: 

Let samplerLayout be a new GPUSamplerBindingLayout . 

Set samplerLayout . type to "comparison" . 

Set entry . sampler to samplerLayout . 

If resource is for a buffer binding: 

Let bufferLayout be a new GPUBufferBindingLayout . 

Set bufferLayout . minBindingSize to resource ’s minimum buffer binding size . 

If resource is for a read-only storage buffer: 

Set bufferLayout . type to "read-only-storage" . 

If resource is for a storage buffer: 

Set bufferLayout . type to "storage" . 

Set entry . buffer to bufferLayout . 

If resource is for a sampled texture binding: 

Let textureLayout be a new GPUTextureBindingLayout . 

If resource is a depth texture binding: 

Set textureLayout . sampleType to "depth" 

Otherwise, if the sampled type of resource is: 

f32 and there exists a static use of resource by stageDesc in a texture builtin function call that also uses a sampler

Set textureLayout . sampleType to "float" 

f32 otherwise

Set textureLayout . sampleType to "unfilterable-float" 

i32 

Set textureLayout . sampleType to "sint" 

u32 

Set textureLayout . sampleType to "uint" 

Set textureLayout . viewDimension to resource ’s dimension. 

If resource is for a multisampled texture: 

Set textureLayout . multisampled to true . 

Set entry . texture to textureLayout . 

If resource is for a storage texture binding: 

Let storageTextureLayout be a new GPUStorageTextureBindingLayout . 

Set storageTextureLayout . format to resource ’s format. 

Set storageTextureLayout . viewDimension to resource ’s dimension. 

If the access mode is: 

read 

Set textureLayout . access to "read-only" . 

write 

Set textureLayout . access to "write-only" . 

read_write 

Set textureLayout . access to "read-write" . 

Set entry . storageTexture to storageTextureLayout . 

Set groupCount to max( groupCount , group + 1). 

If groupDescs [ group ] has an entry previousEntry with binding equal to binding : 

If entry has different visibility than previousEntry : 

Add the bits set in entry . visibility into previousEntry . visibility 

If resource is for a buffer binding and entry has greater
buffer . minBindingSize 
than previousEntry : 

Set previousEntry . buffer . minBindingSize 
to entry . buffer . minBindingSize . 

If resource is a sampled texture binding and entry has different
texture . sampleType than previousEntry 
and both entry and previousEntry have texture . sampleType 
of either "float" or "unfilterable-float" : 

Set previousEntry . texture . sampleType to
"float" . 

If any other property is unequal between entry and previousEntry : 

Return null (which will cause the creation of the pipeline to fail). 

If resource is a storage texture binding,
entry . storageTexture . access is "read-write" ,
previousEntry . storageTexture . access is "write-only" , and
previousEntry . storageTexture . format is compatible with
STORAGE_BINDING and "read-write" according to the § 26.1.1 Plain color formats table: 

Set previousEntry . storageTexture . access to "read-write" . 

Otherwise: 

Append entry to groupDescs [ group ]. 

Let groupLayouts be a new list . 

For each i from 0 to groupCount - 1, inclusive: 

Let groupDesc be groupDescs [ i ]. 

Let bindGroupLayout be the result of calling device . createBindGroupLayout() ( groupDesc ). 

Set bindGroupLayout . [[exclusivePipeline]] to pipeline . 

Append bindGroupLayout to groupLayouts . 

Let desc be a new GPUPipelineLayoutDescriptor . 

Set desc . bindGroupLayouts to groupLayouts . 

Return device . createPipelineLayout() ( desc ). 

10.1.2. GPUProgrammableStage 

A GPUProgrammableStage describes the entry point in the user-provided
GPUShaderModule that controls one of the programmable stages of a pipeline .
Entry point names follow the rules defined in WGSL identifier comparison . 

dictionary GPUProgrammableStage {
required GPUShaderModule module ;
USVString entryPoint ;
record < USVString , GPUPipelineConstantValue > " href="#dom-gpuprogrammablestage-constants" id="ref-for-dom-gpuprogrammablestage-constants①"> constants = {};
};

typedef double GPUPipelineConstantValue ; // May represent WGSL's bool, f32, i32, u32, and f16 if enabled.

GPUProgrammableStage has the following members: 

module , of type GPUShaderModule 

The GPUShaderModule containing the code that this programmable stage will execute. 

entryPoint , of type USVString 

The name of the function in module that this stage will use to
perform its work. 

NOTE: Since the entryPoint dictionary member is
not required, methods which consume a GPUProgrammableStage must use the
" get the entry point " algorithm to determine which entry point
it refers to. 

constants , of type record< USVString , GPUPipelineConstantValue >, defaulting to {} 

Specifies the values of pipeline-overridable constants in the shader module
module . 

Each such pipeline-overridable constant is uniquely identified by a single
pipeline-overridable constant identifier string , representing the pipeline constant ID of the constant if its declaration specifies one, and otherwise the
constant’s identifier name. 

The key of each key-value pair must equal the
identifier string 
of one such constant, with the comparison performed
according to the rules for WGSL identifier comparison .
When the pipeline is executed, that constant will have the specified value. 

Values are specified as GPUPipelineConstantValue , which is a double .
They are converted to WGSL type of the pipeline-overridable constant ( bool / i32 / u32 / f32 / f16 ).
If conversion fails, a validation error is generated. 

Pipeline-overridable constants defined in WGSL:

@id ( 0 ) override has_point_light : bool = true ; // Algorithmic control. 
@id ( 1200 ) override specular_param : f32 = 2.3 ; // Numeric control. 
@id ( 1300 ) override gain : f32 ; // Must be overridden. 
override width : f32 = 0.0 ; // Specifed at the API level 
// using the name "width". 
override depth : f32 ; // Specifed at the API level 
// using the name "depth". 
// Must be overridden. 
override height = 2 * depth ; // The default value 
// (if not set at the API level), 
// depends on another 
// overridable constant. 

Corresponding JavaScript code, providing only the overrides which are required
(have no defaults): 

{ 
// ... 
constants : { 
1300 : 2.0 , // "gain" 
depth : - 1 , // "depth" 
} 
} 

Corresponding JavaScript code, overriding all constants: 

{ 
// ... 
constants : { 
0 : false , // "has_point_light" 
1200 : 3.0 , // "specular_param" 
1300 : 2.0 , // "gain" 
width : 20 , // "width" 
depth : - 1 , // "depth" 
height : 15 , // "height" 
} 
} 

To get the entry point ( GPUShaderStage stage ,
GPUProgrammableStage descriptor ), run the following device timeline steps:

If descriptor . entryPoint is provided : 

If descriptor . module contains an entry point
whose name equals descriptor . entryPoint ,
and whose shader stage equals stage ,
return that entry point. 

Otherwise, return null . 

Otherwise: 

If there is exactly one entry point in descriptor . module 
whose shader stage equals stage , return that entry point. 

Otherwise, return null . 

validating GPUProgrammableStage ( stage , descriptor , layout , device )

Arguments: 

GPUShaderStage stage 

GPUProgrammableStage descriptor 

GPUPipelineLayout layout 

GPUDevice device 

All of the requirements in the following steps must be met.
If any are unmet, return false ; otherwise, return true . 

descriptor . module must be valid to use with device . 

Let entryPoint be get the entry point ( stage , descriptor ). 

entryPoint must not be null . 

For each binding that is statically used by entryPoint : 

validating shader binding ( binding , layout ) must return true . 

For each call call to a texture builtin function in any of the
functions in the shader stage rooted at entryPoint : 

Let textureBinding be the texture binding used in call . 

If textureBinding is of type sampled texture or depth texture and
call uses a sampler binding samplerBinding of type sampler (excluding sampler_comparison ): 

Let texture be the GPUBindGroupLayoutEntry corresponding to textureBinding . 

Let sampler be the GPUBindGroupLayoutEntry corresponding to samplerBinding . 

If sampler . type is "filtering" ,
then texture . sampleType must be
"float" . 

Note: "comparison" samplers can also only be used with
"depth" textures, because they are the only texture type that can
be bound to WGSL texture_depth_* bindings. 

If device . [[features]] does not contain "core-features-and-limits" :

If call is a call to textureLoad ,
textureBinding must not be of type depth texture . 

If call uses a sampler binding samplerBinding and textureBinding is of type depth texture ,
samplerBinding must be of sampler_comparison type. 

For each key → value in descriptor . constants : 

key must equal the pipeline-overridable constant identifier string of
some pipeline-overridable constant defined in the shader module
descriptor . module by the rules defined in WGSL identifier comparison .
The pipeline-overridable constant is not required to be statically used by entryPoint .
Let the type of that constant be T . 

Converting the IDL value value to WGSL type T must not throw a TypeError . 

For each pipeline-overridable constant identifier string key which is
statically used by entryPoint : 

If the pipeline-overridable constant identified by key 
does not have a default value ,
descriptor . constants must contain key . 

Pipeline-creation program errors must not
result from the rules of the [WGSL] specification. 

If device . [[features]] does not contain "core-features-and-limits" :

Let sum be 0. 

For each unique texture or external texture binding textureBinding that is used in any call to a texture builtin in any of the functions in the shader stage rooted at entryPoint : 

Let samplerBindings be the set of sampler bindings used together with textureBinding in any call to a texture builtin in any of the functions in the shader stage rooted at entryPoint . 

Let numPairs be max(1, number of elements of samplerBindings ) . 

If textureBinding is an external texture binding: 

Let numPairs be 1 + 3 * numPairs . 

Let sum be sum + numPairs . 

sum must be ≤ device .limits. maxSampledTexturesPerShaderStage . 

sum must be ≤ device .limits. maxSamplersPerShaderStage . 

validating shader binding ( variable , layout )

Arguments: 

shader binding declaration variable , a module-scope variable declaration reflected from a shader module 

GPUPipelineLayout layout 

Let bindGroup be the bind group index, and bindIndex be the binding index,
of the shader binding declaration variable . 

Return true if all of the following conditions are satisfied: 

layout . [[bindGroupLayouts]] [ bindGroup ] contains
a GPUBindGroupLayoutEntry entry whose entry . binding == bindIndex . 

If the defined binding member for entry is: 

buffer 

If entry . buffer . type is: 

"uniform" 

variable is declared with address space uniform . 

"storage" 

variable is declared with address space storage and access mode read_write . 

"read-only-storage" 

variable is declared with address space storage and access mode read . 

If entry . buffer . minBindingSize is not 0 ,
then it must be at least the minimum buffer binding size for the associated
buffer binding variable in the shader. 

sampler 

If entry . sampler . type is: 

"filtering" or "non-filtering" 

variable has type sampler . 

"comparison" 

variable has type sampler_comparison . 

texture 

If, and only if,
entry . texture . multisampled 
is true , variable has type texture_multisampled_2d<T> or texture_depth_multisampled_2d<T> . 

If entry . texture . sampleType is: 

"float" , "unfilterable-float" ,
"sint" or "uint" 

variable has one of the types: 

texture_1d<T> 

texture_2d<T> 

texture_2d_array<T> 

texture_cube<T> 

texture_cube_array<T> 

texture_3d<T> 

texture_multisampled_2d<T> 

If entry . texture . sampleType is: 

"float" or "unfilterable-float" 

The sampled type T is f32 . 

"sint" 

The sampled type T is i32 . 

"uint" 

The sampled type T is u32 . 

"depth" 

variable has one of the types: 

texture_2d<T> 

texture_2d_array<T> 

texture_cube<T> 

texture_cube_array<T> 

texture_multisampled_2d<T> 

texture_depth_2d 

texture_depth_2d_array 

texture_depth_cube 

texture_depth_cube_array 

texture_depth_multisampled_2d 

where the sampled type T is f32 . 

If entry . texture . viewDimension is: 

"1d" 

variable has type texture_1d<T> . 

"2d" 

variable has type texture_2d<T> or texture_multisampled_2d<T> . 

"2d-array" 

variable has type texture_2d_array<T> . 

"cube" 

variable has type texture_cube<T> . 

"cube-array" 

variable has type texture_cube_array<T> . 

"3d" 

variable has type texture_3d<T> . 

storageTexture 

If entry . storageTexture . viewDimension is: 

"1d" 

variable has type texture_storage_1d<T, A> . 

"2d" 

variable has type texture_storage_2d<T, A> . 

"2d-array" 

variable has type texture_storage_2d_array<T, A> . 

"3d" 

variable has type texture_storage_3d<T, A> . 

If entry . storageTexture . access is: 

"write-only" 

The access mode A is write . 

"read-only" 

The access mode A is read . 

"read-write" 

The access mode A is read_write or write . 

The texel format T equals
entry . storageTexture . format . 

The minimum buffer binding size for a buffer binding variable var is computed as follows:

Let T be the store type of var . 

If T is a runtime-sized array, or contains a runtime-sized array, replace
that array<E> with array<E, 1> . 

Note: This ensures there’s always enough memory for one element, which allows array
indices to be clamped to the length of the array resulting in an in-memory access. 

Return SizeOf ( T ). 

Note: 
Enforcing this lower bound ensures reads and writes via the buffer variable only access memory locations
within the bound region of the buffer. 

A resource binding, pipeline-overridable constant, shader stage input, or shader stage output
is considered to be statically used 
by an entry point if it is present in the interface of the shader stage for that entry point.

10.2. GPUComputePipeline 

A GPUComputePipeline is a kind of pipeline that controls the compute shader stage,
and can be used in GPUComputePassEncoder . 

Compute inputs and outputs are all contained in the bindings,
according to the given GPUPipelineLayout .
The outputs correspond to buffer bindings with a type of "storage" 
and storageTexture bindings with a type of
"write-only" or
"read-write" . 

Stages of a compute pipeline : 

Compute shader 

[ Exposed =( Window , Worker ), SecureContext ]
interface GPUComputePipeline {
};
GPUComputePipeline includes GPUObjectBase ;
GPUComputePipeline includes GPUPipelineBase ;

10.2.1. Compute Pipeline Creation 

A GPUComputePipelineDescriptor describes a compute pipeline . See
§ 23.1 Computing for additional details. 

dictionary GPUComputePipelineDescriptor 
: GPUPipelineDescriptorBase {
required GPUProgrammableStage compute ;
};

GPUComputePipelineDescriptor has the following members: 

compute , of type GPUProgrammableStage 

Describes the compute shader entry point of the pipeline . 

createComputePipeline(descriptor) 

Creates a GPUComputePipeline using immediate pipeline creation . 

Called on: GPUDevice this .

Arguments: 

Arguments for the GPUDevice.createComputePipeline(descriptor) method. 

Parameter
Type
Nullable
Optional
Description

descriptor 
GPUComputePipelineDescriptor 
✘ 
✘ 
Description of the GPUComputePipeline to create.

Returns: GPUComputePipeline 

Content timeline steps: 

Let pipeline be ! create a new WebGPU object ( this , GPUComputePipeline , descriptor ). 

Issue the initialization steps on the Device timeline of this . 

Return pipeline . 

Device timeline initialization steps :

Let layout be a new default pipeline layout for pipeline if
descriptor . layout is "auto" ,
and descriptor . layout otherwise. 

All of the requirements in the following steps must be met.
If any are unmet, generate a validation error , invalidate pipeline and return. 

layout must be valid to use with this . 

validating GPUProgrammableStage ( COMPUTE ,
descriptor . compute , layout , this ) must succeed. 

Let entryPoint be get the entry point ( COMPUTE , descriptor . compute ). 

Assert entryPoint is not null . 

Let workgroupStorageUsed be the sum of roundUp (16, SizeOf ( T )) over each
type T of all variables with address space " workgroup "
statically used by entryPoint . 

workgroupStorageUsed must be ≤
device .limits. maxComputeWorkgroupStorageSize . 

entryPoint must use ≤
device .limits. maxComputeInvocationsPerWorkgroup per
workgroup. 

Each component of entryPoint ’s
workgroup_size attribute must be ≤ the corresponding component in
[ device .limits. maxComputeWorkgroupSizeX ,
device .limits. maxComputeWorkgroupSizeY ,
device .limits. maxComputeWorkgroupSizeZ ]. 

If any pipeline-creation uncategorized errors 
result from the implementation of pipeline creation,
generate an internal error , invalidate pipeline and return. 

Note: 
Even if the implementation detected uncategorized errors in shader module
creation, the error is surfaced here. 

Set pipeline . [[layout]] to layout . 

createComputePipelineAsync(descriptor) 

Creates a GPUComputePipeline using async pipeline creation .
The returned Promise resolves when the created pipeline
is ready to be used without additional delay. 

If pipeline creation fails, the returned Promise rejects with an GPUPipelineError .
(A GPUError is not dispatched to the device.) 

Note: Use of this method is preferred whenever possible, as it prevents blocking the
queue timeline work on pipeline compilation. 

Called on: GPUDevice this .

Arguments: 

Arguments for the GPUDevice.createComputePipelineAsync(descriptor) method. 

Parameter
Type
Nullable
Optional
Description

descriptor 
GPUComputePipelineDescriptor 
✘ 
✘ 
Description of the GPUComputePipeline to create.

Returns: Promise < GPUComputePipeline > 

Content timeline steps: 

Let contentTimeline be the current Content timeline . 

Let promise be a new promise . 

Issue the initialization steps on the Device timeline of this . 

Return promise . 

Device timeline initialization steps :

Let pipeline be a new GPUComputePipeline created as if
this . createComputePipeline() was called with descriptor ,
except capturing any errors as error , rather than dispatching them to the device. 

Let event occur upon the (successful or unsuccessful) completion of
pipeline creation for pipeline . 

Listen for timeline event event 
on this . [[device]] , handled by
the subsequent steps on the device timeline of this . 

Device timeline steps:

If pipeline is valid or this is lost : 

Issue the following steps on contentTimeline : 

Content timeline steps:

Resolve promise with pipeline . 

Return. 

Note: No errors are generated from a device which is lost.
See § 22 Errors & Debugging . 

If pipeline is invalid and error is an internal error ,
issue the following steps on contentTimeline , and return. 

Content timeline steps:

Reject promise with a GPUPipelineError with
reason "internal" . 

If pipeline is invalid and error is a validation error ,
issue the following steps on contentTimeline , and return. 

Content timeline steps:

Reject promise with a GPUPipelineError with
reason "validation" . 

Creating a simple GPUComputePipeline :

const computePipeline = gpuDevice . createComputePipeline ({ 
layout : pipelineLayout , 
compute : { 
module : computeShaderModule , 
entryPoint : 'computeMain' , 
} 
}); 

10.3. GPURenderPipeline 

A GPURenderPipeline is a kind of pipeline that controls the vertex
and fragment shader stages, and can be used in GPURenderPassEncoder 
as well as GPURenderBundleEncoder . 

Render pipeline inputs are: 

bindings, according to the given GPUPipelineLayout 

vertex and index buffers, described by GPUVertexState 

the color attachments, described by GPUColorTargetState 

optionally, the depth-stencil attachment, described by GPUDepthStencilState 

Render pipeline outputs are: 

buffer bindings with a type of "storage" 

storageTexture bindings with a access of "write-only" or "read-write" 

the color attachments, described by GPUColorTargetState 

optionally, depth-stencil attachment, described by GPUDepthStencilState 

A render pipeline is comprised of the following render stages : 

Vertex fetch, controlled by GPUVertexState.buffers 

Vertex shader, controlled by GPUVertexState 

Primitive assembly, controlled by GPUPrimitiveState 

Rasterization, controlled by GPUPrimitiveState , GPUDepthStencilState , and GPUMultisampleState 

Fragment shader, controlled by GPUFragmentState 

Stencil test and operation, controlled by GPUDepthStencilState 

Depth test and write, controlled by GPUDepthStencilState 

Output merging, controlled by GPUFragmentState.targets 

[ Exposed =( Window , Worker ), SecureContext ]
interface GPURenderPipeline {
};
GPURenderPipeline includes GPUObjectBase ;
GPURenderPipeline includes GPUPipelineBase ;

GPURenderPipeline has the following device timeline properties : 

[[descriptor]] , of type GPURenderPipelineDescriptor , readonly

The GPURenderPipelineDescriptor describing this pipeline. 

All optional fields of GPURenderPipelineDescriptor are defined. 

[[writesDepth]] , of type boolean , readonly

True if the pipeline writes to the depth component of the depth/stencil attachment 

[[writesStencil]] , of type boolean , readonly

True if the pipeline writes to the stencil component of the depth/stencil attachment 

10.3.1. Render Pipeline Creation 

A GPURenderPipelineDescriptor describes a render pipeline by configuring each
of the render stages . See § 23.2 Rendering for additional details. 

dictionary GPURenderPipelineDescriptor 
: GPUPipelineDescriptorBase {
required GPUVertexState vertex ;
GPUPrimitiveState primitive = {};
GPUDepthStencilState depthStencil ;
GPUMultisampleState multisample = {};
GPUFragmentState fragment ;
};

GPURenderPipelineDescriptor has the following members: 

vertex , of type GPUVertexState 

Describes the vertex shader entry point of the pipeline and its input buffer layouts. 

primitive , of type GPUPrimitiveState , defaulting to {} 

Describes the primitive-related properties of the pipeline . 

depthStencil , of type GPUDepthStencilState 

Describes the optional depth-stencil properties, including the testing, operations, and bias. 

multisample , of type GPUMultisampleState , defaulting to {} 

Describes the multi-sampling properties of the pipeline . 

fragment , of type GPUFragmentState 

Describes the fragment shader entry point of the pipeline and its output colors. If
not provided , the § 23.2.8 No Color Output mode is enabled. 

createRenderPipeline(descriptor) 

Creates a GPURenderPipeline using immediate pipeline creation . 

Called on: GPUDevice this .

Arguments: 

Arguments for the GPUDevice.createRenderPipeline(descriptor) method. 

Parameter
Type
Nullable
Optional
Description

descriptor 
GPURenderPipelineDescriptor 
✘ 
✘ 
Description of the GPURenderPipeline to create.

Returns: GPURenderPipeline 

Content timeline steps: 

If descriptor . fragment is provided : 

For each non- null colorState of
descriptor . fragment . targets : 

? Validate texture format required features of
colorState . format with this . [[device]] . 

If descriptor . depthStencil is provided : 

? Validate texture format required features of
descriptor . depthStencil . format 
with this . [[device]] . 

Let pipeline be ! create a new WebGPU object ( this , GPURenderPipeline , descriptor ). 

Issue the initialization steps on the Device timeline of this . 

Return pipeline . 

Device timeline initialization steps :

Let layout be a new default pipeline layout for pipeline if
descriptor . layout is "auto" ,
and descriptor . layout otherwise. 

All of the requirements in the following steps must be met.
If any are unmet, generate a validation error , invalidate pipeline , and return. 

layout must be valid to use with this . 

validating GPURenderPipelineDescriptor ( descriptor , layout , this ) must succeed. 

Let vertexBufferCount be the index of the last non-null entry in
descriptor . vertex . buffers ,
plus 1; or 0 if there are none. 

layout . [[bindGroupLayouts]] . size + vertexBufferCount must be ≤
this . [[device]] . [[limits]] . maxBindGroupsPlusVertexBuffers . 

If any pipeline-creation uncategorized errors 
result from the implementation of pipeline creation,
generate an internal error , invalidate pipeline and return. 

Note: 
Even if the implementation detected uncategorized errors in shader module
creation, the error is surfaced here. 

Set pipeline . [[descriptor]] to descriptor . 

Set pipeline . [[writesDepth]] to false. 

Set pipeline . [[writesStencil]] to false. 

Let depthStencil be descriptor . depthStencil . 

If depthStencil is not null: 

If depthStencil . depthWriteEnabled is provided : 

Set pipeline . [[writesDepth]] to depthStencil . depthWriteEnabled . 

If depthStencil . stencilWriteMask is not 0: 

Let stencilFront be depthStencil . stencilFront . 

Let stencilBack be depthStencil . stencilBack . 

Let cullMode be descriptor . primitive . cullMode . 

If cullMode is not "front" , and any of stencilFront . passOp ,
stencilFront . depthFailOp , or stencilFront . failOp 
is not "keep" : 

Set pipeline . [[writesStencil]] to true. 

If cullMode is not "back" , and any of stencilBack . passOp ,
stencilBack . depthFailOp , or stencilBack . failOp 
is not "keep" : 

Set pipeline . [[writesStencil]] to true. 

Set pipeline . [[layout]] to layout . 

createRenderPipelineAsync(descriptor) 

Creates a GPURenderPipeline using async pipeline creation .
The returned Promise resolves when the created pipeline
is ready to be used without additional delay. 

If pipeline creation fails, the returned Promise rejects with an GPUPipelineError .
(A GPUError is not dispatched to the device.) 

Note: Use of this method is preferred whenever possible, as it prevents blocking the
queue timeline work on pipeline compilation. 

Called on: GPUDevice this .

Arguments: 

Arguments for the GPUDevice.createRenderPipelineAsync(descriptor) method. 

Parameter
Type
Nullable
Optional
Description

descriptor 
GPURenderPipelineDescriptor 
✘ 
✘ 
Description of the GPURenderPipeline to create.

Returns: Promise < GPURenderPipeline > 

Content timeline steps: 

Let contentTimeline be the current Content timeline . 

Let promise be a new promise . 

Issue the initialization steps on the Device timeline of this . 

Return promise . 

Device timeline initialization steps :

Let pipeline be a new GPURenderPipeline created as if
this . createRenderPipeline() was called with descriptor ,
except capturing any errors as error , rather than dispatching them to the device. 

Let event occur upon the (successful or unsuccessful) completion of
pipeline creation for pipeline . 

Listen for timeline event event 
on this . [[device]] , handled by
the subsequent steps on the device timeline of this . 

Device timeline steps:

If pipeline is valid or this is lost : 

Issue the following steps on contentTimeline : 

Content timeline steps:

Resolve promise with pipeline . 

Return. 

Note: No errors are generated from a device which is lost.
See § 22 Errors & Debugging . 

If pipeline is invalid and error is an internal error ,
issue the following steps on contentTimeline , and return. 

Content timeline steps:

Reject promise with a GPUPipelineError with
reason "internal" . 

If pipeline is invalid and error is a validation error ,
issue the following steps on contentTimeline , and return. 

Content timeline steps:

Reject promise with a GPUPipelineError with
reason "validation" . 

validating GPURenderPipelineDescriptor (descriptor, layout, device)

Arguments: 

GPURenderPipelineDescriptor descriptor 

GPUPipelineLayout layout 

GPUDevice device 

Device timeline steps: 

Return true if all of the following conditions are satisfied: 

validating GPUVertexState ( device , descriptor . vertex , layout ) succeeds. 

If descriptor . fragment is provided : 

validating GPUFragmentState ( device , descriptor . fragment , layout ) succeeds. 

If the sample_mask builtin is a shader stage output of
descriptor . fragment : 

descriptor . multisample . alphaToCoverageEnabled is false . 

If the frag_depth builtin is a shader stage output of
descriptor . fragment : 

descriptor . depthStencil must be
provided , and
descriptor . depthStencil . format 
must have a depth aspect. 

If device . [[features]] does not contain "core-features-and-limits" :

The sample_mask builtin must not be a shader stage input or shader stage output of descriptor . fragment . 

The sample_index builtin must not be a shader stage input of descriptor . fragment . 

validating GPUPrimitiveState ( descriptor . primitive , device ) succeeds. 

If descriptor . depthStencil is provided : 

validating GPUDepthStencilState ( device , descriptor . depthStencil ,
descriptor . primitive . topology ) succeeds. 

validating GPUMultisampleState ( descriptor . multisample ) succeeds. 

If descriptor . multisample . alphaToCoverageEnabled 
is true: 

descriptor . fragment must be provided . 

descriptor . fragment . targets [0]
must exist and be non-null. 

descriptor . fragment . targets [0]. format 
must be a GPUTextureFormat which is blendable and has an alpha channel. 

There must exist at least one attachment, either: 

A non- null value in
descriptor . fragment . targets , or 

A descriptor . depthStencil . 

validating inter-stage interfaces ( device , descriptor ) returns true . 

validating Compatibility Mode shader binding ( variable )

Arguments: 

shader binding declaration variable , a module-scope variable declaration reflected from a shader module 

Returns: boolean 

Device timeline steps: 

If the interpolation of the variable is linear , return false . 

If the interpolation of the variable is flat and the interpolation sampling is not either , return false . 

If the interpolation sampling of the variable is sample , return false . 

Return 'true' 

validating inter-stage interfaces ( device , descriptor )

Arguments: 

GPUDevice device 

GPURenderPipelineDescriptor descriptor 

Returns: boolean 

Device timeline steps: 

Let maxVertexShaderOutputVariables be
device .limits. maxInterStageShaderVariables . 

Let maxVertexShaderOutputLocation be
device .limits. maxInterStageShaderVariables - 1. 

If descriptor . primitive . topology 
is "point-list" : 

Decrement maxVertexShaderOutputVariables by 1. 

If clip_distances is declared in the output of
descriptor . vertex : 

Let clipDistancesSize be the array size of clip_distances . 

Decrement maxVertexShaderOutputVariables by ceil( clipDistancesSize / 4). 

Decrement maxVertexShaderOutputLocation by ceil( clipDistancesSize / 4). 

Return false if any of the following requirements are unmet: 

There must be no more than maxVertexShaderOutputVariables user-defined outputs for
descriptor . vertex . 

The location of each user-defined output of
descriptor . vertex must be
≤ maxVertexShaderOutputLocation . 

If device . [[features]] does not contain "core-features-and-limits" : 

For each user-defined output of descriptor . vertex : 

If validating Compatibility Mode shader binding ( output ) fails, return false . 

If descriptor . fragment is provided : 

Let maxFragmentShaderInputVariables be
device .limits. maxInterStageShaderVariables . 

For each of the Inter-Stage Builtins that are an input of
descriptor . fragment : 

Decrement maxFragmentShaderInputVariables by 1. 

Return false if any of the following requirements are unmet: 

For each user-defined input of descriptor . fragment there
must be a user-defined output of descriptor . vertex that
location , type, and interpolation of the input. 

Note: Vertex-only pipelines can have user-defined outputs in the vertex stage;
their values will be discarded. 

There must be no more than maxFragmentShaderInputVariables user-defined inputs for
descriptor . fragment . 

Assert that the location of each user-defined input of
descriptor . fragment is less
than device .limits. maxInterStageShaderVariables .
(This follows from the above rules.) 

If device . [[features]] does not contain "core-features-and-limits" : 

For each user-defined input of descriptor . fragment : 

If validating Compatibility Mode shader binding ( input ) fails, return false . 

Return true . 

The following builtins are Inter-Stage Builtins , and count towards the
maxInterStageShaderVariables limit when used in a fragment shader: 

front_facing 

sample_index 

sample_mask 

primitive_index 

subgroup_invocation_id 

subgroup_size 

Creating a simple GPURenderPipeline :

const renderPipeline = gpuDevice . createRenderPipeline ({ 
layout : pipelineLayout , 
vertex : { 
module : shaderModule , 
entryPoint : 'vertexMain' 
}, 
fragment : { 
module : shaderModule , 
entryPoint : 'fragmentMain' , 
targets : [{ 
format : 'bgra8unorm' , 
}], 
} 
}); 

10.3.2. Primitive State 

dictionary GPUPrimitiveState {
GPUPrimitiveTopology topology = "triangle-list";
GPUIndexFormat stripIndexFormat ;
GPUFrontFace frontFace = "ccw";
GPUCullMode cullMode = "none";

// Requires "depth-clip-control" feature.
boolean unclippedDepth = false ;
};

GPUPrimitiveState has the following members, which describe how a GPURenderPipeline 
constructs and rasterizes primitives from its vertex inputs: 

topology , of type GPUPrimitiveTopology , defaulting to "triangle-list" 

The type of primitive to be constructed from the vertex inputs. 

stripIndexFormat , of type GPUIndexFormat 

For pipelines with strip topologies
( "line-strip" or "triangle-strip" ),
this determines the index buffer format and primitive restart value
( "uint16" / 0xFFFF or "uint32" / 0xFFFFFFFF ).
It is not allowed on pipelines with non-strip topologies. 

Note: Some implementations require knowledge of the primitive restart value to compile
pipeline state objects. 

To use a strip-topology pipeline with an indexed draw call
( drawIndexed() or drawIndexedIndirect() ),
this must be set, and it must match the index buffer format used with the draw call
(set in setIndexBuffer() ). 

See § 23.2.3 Primitive Assembly for additional details. 

frontFace , of type GPUFrontFace , defaulting to "ccw" 

Defines which polygons are considered front-facing . 

cullMode , of type GPUCullMode , defaulting to "none" 

Defines which polygon orientation will be culled, if any. 

unclippedDepth , of type boolean , defaulting to false 

If true, indicates that depth clipping is disabled. 

Requires the "depth-clip-control" feature to be enabled. 

validating GPUPrimitiveState ( descriptor , device )
Arguments: 

GPUPrimitiveState descriptor 

GPUDevice device 

Device timeline steps: 

Return true if all of the following conditions are satisfied: 

If descriptor . topology is not
"line-strip" or "triangle-strip" : 

descriptor . stripIndexFormat must not be provided . 

If descriptor . unclippedDepth is true : 

"depth-clip-control" must be enabled for device . 

enum GPUPrimitiveTopology {
"point-list" ,
"line-list" ,
"line-strip" ,
"triangle-list" ,
"triangle-strip" ,
};

GPUPrimitiveTopology defines the primitive type draw calls made with a GPURenderPipeline 
will use. See § 23.2.5 Rasterization for additional details: 

"point-list" 

Each vertex defines a point primitive. 

"line-list" 

Each consecutive pair of two vertices defines a line primitive. 

"line-strip" 

Each vertex after the first defines a line primitive between it and the previous vertex. 

"triangle-list" 

Each consecutive triplet of three vertices defines a triangle primitive. 

"triangle-strip" 

Each vertex after the first two defines a triangle primitive between it and the previous
two vertices. 

enum GPUFrontFace {
"ccw" ,
"cw" ,
};

GPUFrontFace defines which polygons are considered front-facing by a GPURenderPipeline .
See § 23.2.5.4 Polygon Rasterization for additional details: 

"ccw" 

Polygons with vertices whose framebuffer coordinates are given in counter-clockwise order
are considered front-facing . 

"cw" 

Polygons with vertices whose framebuffer coordinates are given in clockwise order are
considered front-facing . 

enum GPUCullMode {
"none" ,
"front" ,
"back" ,
};

GPUPrimitiveTopology defines which polygons will be culled by draw calls made with a
GPURenderPipeline . See § 23.2.5.4 Polygon Rasterization for additional details: 

"none" 

No polygons are discarded. 

"front" 

Front-facing polygons are discarded. 

"back" 

Back-facing polygons are discarded. 

Note: GPUFrontFace and GPUCullMode have no effect on "point-list" ,
"line-list" , or "line-strip" topologies. 

10.3.3. Multisample State 

dictionary GPUMultisampleState {
GPUSize32 count = 1;
GPUSampleMask mask = 0xFFFFFFFF;
boolean alphaToCoverageEnabled = false ;
};

GPUMultisampleState has the following members, which describe how a GPURenderPipeline 
interacts with a render pass’s multisampled attachments. 

count , of type GPUSize32 , defaulting to 1 

Number of samples per pixel. This GPURenderPipeline will be compatible only
with attachment textures ( colorAttachments 
and depthStencilAttachment )
with matching sampleCount s. 

mask , of type GPUSampleMask , defaulting to 0xFFFFFFFF 

Mask determining which samples are written to. 

alphaToCoverageEnabled , of type boolean , defaulting to false 

When true indicates that a fragment’s alpha channel should be used to generate a sample
coverage mask. 

validating GPUMultisampleState ( descriptor )
Arguments: 

GPUMultisampleState descriptor 

Device timeline steps: 

Return true if all of the following conditions are satisfied: 

descriptor . count must be either 1 or 4. 

If descriptor . alphaToCoverageEnabled is true : 

descriptor . count > 1. 

10.3.4. Fragment State 

dictionary GPUFragmentState 
: GPUProgrammableStage {
required sequence < GPUColorTargetState ?> " href="#dom-gpufragmentstate-targets" id="ref-for-dom-gpufragmentstate-targets⑥"> targets ;
};

targets , of type sequence<GPUColorTargetState?> 

A list of GPUColorTargetState defining the formats and behaviors of the color targets
this pipeline writes to. 

validating GPUFragmentState ( device , descriptor , layout )

Arguments: 

GPUDevice device 

GPUFragmentState descriptor 

GPUPipelineLayout layout 

Device timeline steps: 

Return true if all of the following requirements are met: 

validating GPUProgrammableStage ( FRAGMENT , descriptor , layout , device ) succeeds. 

descriptor . targets . size must be ≤
device . [[limits]] . maxColorAttachments . 

For each shader stage output output : 

output ’s location must be < device . [[limits]] . maxColorAttachments . 

Let entryPoint be get the entry point ( FRAGMENT , descriptor ). 

Let usesDualSourceBlending be false . 

For each index of the indices of descriptor . targets 
containing a non- null value colorState : 

colorState . format must be listed in § 26.1.1 Plain color formats 
with RENDER_ATTACHMENT capability. 

colorState . writeMask must be < 16. 

If colorState . blend is provided : 

The colorState . format must be blendable . 

colorState . blend . color 
must be a valid GPUBlendComponent . 

colorState . blend . alpha 
must be a valid GPUBlendComponent . 

If colorState . blend . color . srcFactor or
colorState . blend . color . dstFactor or
colorState . blend . alpha . srcFactor or
colorState . blend . alpha . dstFactor 
uses the second input of the corresponding blending unit (is any of
"src1" , "one-minus-src1" ,
"src1-alpha" , "one-minus-src1-alpha" ), then: 

Set usesDualSourceBlending to true . 

For each shader stage output value output with location attribute equal to index 
in entryPoint : 

For each component in colorState . format , there must be a
corresponding component in output .
(That is, RGBA requires vec4, RGB requires vec3 or vec4, RG requires vec2 or vec3 or vec4.) 

If the GPUTextureSampleType s for colorState . format 
(defined in § 26.1 Texture Format Capabilities ) are: 

"float" and/or "unfilterable-float" 

output must have a floating-point scalar type. 

"sint" 

output must have a signed integer scalar type. 

"uint" 

output must have an unsigned integer scalar type. 

If colorState . blend is provided and
colorState . blend . color . srcFactor 
or . dstFactor uses the source alpha
(is any of "src-alpha" , "one-minus-src-alpha" ,
"src-alpha-saturated" , "src1-alpha" or
"one-minus-src1-alpha" ), then: 

output must have an alpha channel (that is, it must be a vec4). 

If colorState . writeMask is not 0: 

entryPoint must have a shader stage output with location equal to index 
and blend_src omitted or equal to 0. 

If usesDualSourceBlending is true : 

descriptor . targets . size must be 1. 

All the shader stage outputs with location in entryPoint must be in one
struct and use dual source blending . 

Validating GPUFragmentState’s color attachment bytes per sample ( device , descriptor . targets ) succeeds. 

If device . [[features]] does not contain "core-features-and-limits" :

All non-null GPUColorTargetState s colorState in
descriptor . targets must have equal values for each
of the following members: 

colorState . blend . color 

colorState . blend . alpha 

colorState . writeMask 

For each function in the functions in the shader stage rooted at entryPoint : 

function must not be dpdxFine , dpdyFine , or fwidthFine . 

Validating GPUFragmentState’s color attachment bytes per sample ( device , targets )

Arguments: 

GPUDevice device 

sequence < GPUColorTargetState ?> targets 

Device timeline steps: 

Let formats be an empty list < GPUTextureFormat ?> 

For each target in targets : 

If target is undefined , continue. 

Append target . format to formats . 

Calculating color attachment bytes per sample ( formats ) must be ≤ device . [[limits]] . maxColorAttachmentBytesPerSample . 

Note: 
The fragment shader may output more values than what the pipeline uses. If that is the case
the values are ignored. 

GPUBlendComponent component is a valid GPUBlendComponent with logical
device device if it meets

the following requirements:

If component . operation is
"min" or "max" : 

component . srcFactor and
component . dstFactor must both be "one" . 

If component . srcFactor or
component . dstFactor requires a feature according to the
GPUBlendFactor table and device . [[features]] does not contain 
the feature: 

Throw a TypeError . 

10.3.5. Color Target State 

dictionary GPUColorTargetState {
required GPUTextureFormat format ;

GPUBlendState blend ;
GPUColorWriteFlags writeMask = 0xF; // GPUColorWrite.ALL
};

format , of type GPUTextureFormat 

The GPUTextureFormat of this color target. The pipeline will only be compatible with
GPURenderPassEncoder s which use a GPUTextureView of this format in the
corresponding color attachment. 

blend , of type GPUBlendState 

The blending behavior for this color target. If left undefined, disables blending for this
color target. 

writeMask , of type GPUColorWriteFlags , defaulting to 0xF 

Bitmask controlling which channels are are written to when drawing to this color target. 

dictionary GPUBlendState {
required GPUBlendComponent color ;
required GPUBlendComponent alpha ;
};

color , of type GPUBlendComponent 

Defines the blending behavior of the corresponding render target for color channels. 

alpha , of type GPUBlendComponent 

Defines the blending behavior of the corresponding render target for the alpha channel. 

typedef [ EnforceRange ] unsigned long GPUColorWriteFlags ;
[ Exposed =( Window , Worker ), SecureContext ]
namespace GPUColorWrite {
const GPUFlagsConstant RED = 0x1;
const GPUFlagsConstant GREEN = 0x2;
const GPUFlagsConstant BLUE = 0x4;
const GPUFlagsConstant ALPHA = 0x8;
const GPUFlagsConstant ALL = 0xF;
};

10.3.5.1. Blend State 

dictionary GPUBlendComponent {
GPUBlendOperation operation = "add";
GPUBlendFactor srcFactor = "one";
GPUBlendFactor dstFactor = "zero";
};

GPUBlendComponent has the following members, which describe how the color or alpha components
of a fragment are blended: 

operation , of type GPUBlendOperation , defaulting to "add" 

Defines the GPUBlendOperation used to calculate the values written to the target
attachment components. 

srcFactor , of type GPUBlendFactor , defaulting to "one" 

Defines the GPUBlendFactor operation to be performed on values from the fragment shader. 

dstFactor , of type GPUBlendFactor , defaulting to "zero" 

Defines the GPUBlendFactor operation to be performed on values from the target attachment. 

The following tables use this notation to describe color components for a given fragment
location: 

RGBA src 

Color output by the fragment shader for the color attachment.
If the shader doesn’t return an alpha channel, src-alpha blend factors cannot be used.

RGBA src1 

Color output by the fragment shader for the color attachment with
"@blend_src" attribute 
equal to 1 .
If the shader doesn’t return an alpha channel, src1-alpha blend factors cannot be used.

RGBA dst 

Color currently in the color attachment.
Missing green/blue/alpha channels default to 0, 0, 1 , respectively.

RGBA const 

The current [[blendConstant]] .

RGBA srcFactor 

The source blend factor components, as defined by srcFactor .

RGBA dstFactor 

The destination blend factor components, as defined by dstFactor .

enum GPUBlendFactor {
"zero" ,
"one" ,
"src" ,
"one-minus-src" ,
"src-alpha" ,
"one-minus-src-alpha" ,
"dst" ,
"one-minus-dst" ,
"dst-alpha" ,
"one-minus-dst-alpha" ,
"src-alpha-saturated" ,
"constant" ,
"one-minus-constant" ,
"src1" ,
"one-minus-src1" ,
"src1-alpha" ,
"one-minus-src1-alpha" ,
};

GPUBlendFactor defines how either a source or destination blend factors is calculated: 

GPUBlendFactor

Blend factor RGBA components

Feature 

"zero" 

(0, 0, 0, 0) 

"one" 

(1, 1, 1, 1) 

"src" 

(R src , G src , B src , A src ) 

"one-minus-src" 

(1 - R src , 1 - G src , 1 - B src , 1 - A src ) 

"src-alpha" 

(A src , A src , A src , A src ) 

"one-minus-src-alpha" 

(1 - A src , 1 - A src , 1 - A src , 1 - A src ) 

"dst" 

(R dst , G dst , B dst , A dst ) 

"one-minus-dst" 

(1 - R dst , 1 - G dst , 1 - B dst , 1 - A dst ) 

"dst-alpha" 

(A dst , A dst , A dst , A dst ) 

"one-minus-dst-alpha" 

(1 - A dst , 1 - A dst , 1 - A dst , 1 - A dst ) 

"src-alpha-saturated" 

(min(A src , 1 - A dst ), min(A src , 1 - A dst ), min(A src , 1 - A dst ), 1) 

"constant" 

(R const , G const , B const , A const ) 

"one-minus-constant" 

(1 - R const , 1 - G const , 1 - B const , 1 - A const ) 

"src1" 

(R src1 , G src1 , B src1 , A src1 ) 

dual-source-blending 

"one-minus-src1" 

(1 - R src1 , 1 - G src1 , 1 - B src1 , 1 - A src1 ) 

"src1-alpha" 

(A src1 , A src1 , A src1 , A src1 ) 

"one-minus-src1-alpha" 

(1 - A src1 , 1 - A src1 , 1 - A src1 , 1 - A src1 ) 

enum GPUBlendOperation {
"add" ,
"subtract" ,
"reverse-subtract" ,
"min" ,
"max" ,
};

GPUBlendOperation defines the algorithm used to combine source and destination blend factors: 

GPUBlendOperation

RGBA Components

"add" 

RGBA src × RGBA srcFactor + RGBA dst × RGBA dstFactor 

"subtract" 

RGBA src × RGBA srcFactor - RGBA dst × RGBA dstFactor 

"reverse-subtract" 

RGBA dst × RGBA dstFactor - RGBA src × RGBA srcFactor 

"min" 

min(RGBA src , RGBA dst ) 

"max" 

max(RGBA src , RGBA dst ) 

10.3.6. Depth/Stencil State 

dictionary GPUDepthStencilState {
required GPUTextureFormat format ;

boolean depthWriteEnabled ;
GPUCompareFunction depthCompare ;

GPUStencilFaceState stencilFront = {};
GPUStencilFaceState stencilBack = {};

GPUStencilValue stencilReadMask = 0xFFFFFFFF;
GPUStencilValue stencilWriteMask = 0xFFFFFFFF;

GPUDepthBias depthBias = 0;
float depthBiasSlopeScale = 0;
float depthBiasClamp = 0;
};

GPUDepthStencilState has the following members, which describe how a GPURenderPipeline 
will affect a render pass’s depthStencilAttachment : 

format , of type GPUTextureFormat 

The format of depthStencilAttachment 
this GPURenderPipeline will be compatible with. 

depthWriteEnabled , of type boolean 

Indicates if this GPURenderPipeline can modify
depthStencilAttachment depth values. 

depthCompare , of type GPUCompareFunction 

The comparison operation used to test fragment depths against
depthStencilAttachment depth values. 

stencilFront , of type GPUStencilFaceState , defaulting to {} 

Defines how stencil comparisons and operations are performed for front-facing primitives. 

stencilBack , of type GPUStencilFaceState , defaulting to {} 

Defines how stencil comparisons and operations are performed for back-facing primitives. 

stencilReadMask , of type GPUStencilValue , defaulting to 0xFFFFFFFF 

Bitmask controlling which depthStencilAttachment stencil value
bits are read when performing stencil comparison tests. 

stencilWriteMask , of type GPUStencilValue , defaulting to 0xFFFFFFFF 

Bitmask controlling which depthStencilAttachment stencil value
bits are written to when performing stencil operations. 

depthBias , of type GPUDepthBias , defaulting to 0 

Constant depth bias added to each triangle fragment. See biased fragment depth for details. 

depthBiasSlopeScale , of type float , defaulting to 0 

Depth bias that scales with the triangle fragment’s slope. See biased fragment depth for details. 

depthBiasClamp , of type float , defaulting to 0 

The maximum depth bias of a triangle fragment. See biased fragment depth for details. 

Note: depthBias , depthBiasSlopeScale , and
depthBiasClamp have no effect on "point-list" ,
"line-list" , and "line-strip" primitives, and
must be 0. 

The biased fragment depth for a fragment being written to
depthStencilAttachment attachment when drawing using
GPUDepthStencilState state is calculated by running the following queue timeline steps:

Let format be attachment . view . format . 

Let r be the minimum positive representable value > 0 in the format converted to a 32-bit float. 

Let maxDepthSlope be the maximum of the horizontal and vertical slopes of the fragment’s depth value. 

If format is a unorm format: 

Let bias be (float) state . depthBias * r + state . depthBiasSlopeScale * maxDepthSlope . 

Otherwise, if format is a float format: 

Let bias be (float) state . depthBias * 2^(exp(max depth in primitive) - r ) + state . depthBiasSlopeScale * maxDepthSlope . 

If state . depthBiasClamp > 0 : 

Set bias to min( state . depthBiasClamp , bias ) . 

Otherwise, if state . depthBiasClamp < 0 : 

Set bias to max( state . depthBiasClamp , bias ) . 

If state . depthBias ≠ 0 or state . depthBiasSlopeScale ≠ 0 : 

Set the fragment depth value to fragment depth value + bias 

validating GPUDepthStencilState ( device , descriptor , topology )

Arguments: 

GPUDevice device 

GPUDepthStencilState descriptor 

GPUPrimitiveTopology topology 

Device timeline steps: 

Return true if, and only if, all of the following conditions are satisfied: 

descriptor . format is a depth-or-stencil format . 

If descriptor . depthWriteEnabled is true or
descriptor . depthCompare is provided and not
"always" : 

descriptor . format must have a depth component. 

If descriptor . stencilFront or
descriptor . stencilBack are not the default values: 

descriptor . format must have a stencil component. 

If descriptor . format has a depth component: 

descriptor . depthWriteEnabled must be provided . 

descriptor . depthCompare must be provided if: 

descriptor . depthWriteEnabled is true , or 

descriptor . stencilFront . depthFailOp 
is not "keep" , or 

descriptor . stencilBack . depthFailOp 
is not "keep" . 

If topology is "point-list" , "line-list" , or
"line-strip" : 

descriptor . depthBias must be 0. 

descriptor . depthBiasSlopeScale must be 0. 

descriptor . depthBiasClamp must be 0. 

If device . [[features]] does not contain "core-features-and-limits" :

descriptor . depthBiasClamp must be 0. 

dictionary GPUStencilFaceState {
GPUCompareFunction compare = "always";
GPUStencilOperation failOp = "keep";
GPUStencilOperation depthFailOp = "keep";
GPUStencilOperation passOp = "keep";
};

GPUStencilFaceState has the following members, which describe how stencil comparisons and
operations are performed: 

compare , of type GPUCompareFunction , defaulting to "always" 

The GPUCompareFunction used when testing the [[stencilReference]] value
against the fragment’s depthStencilAttachment stencil values. 

failOp , of type GPUStencilOperation , defaulting to "keep" 

The GPUStencilOperation performed if the fragment stencil comparison test described by
compare fails. 

depthFailOp , of type GPUStencilOperation , defaulting to "keep" 

The GPUStencilOperation performed if the fragment depth comparison described by
depthCompare fails. 

passOp , of type GPUStencilOperation , defaulting to "keep" 

The GPUStencilOperation performed if the fragment stencil comparison test described by
compare passes. 

enum GPUStencilOperation {
"keep" ,
"zero" ,
"replace" ,
"invert" ,
"increment-clamp" ,
"decrement-clamp" ,
"increment-wrap" ,
"decrement-wrap" ,
};

GPUStencilOperation defines the following operations: 

"keep" 

Keep the current stencil value. 

"zero" 

Set the stencil value to 0 . 

"replace" 

Set the stencil value to [[stencilReference]] . 

"invert" 

Bitwise-invert the current stencil value. 

"increment-clamp" 

Increments the current stencil value, clamping to the maximum representable value of the
depthStencilAttachment ’s stencil aspect. 

"decrement-clamp" 

Decrement the current stencil value, clamping to 0 . 

"increment-wrap" 

Increments the current stencil value, wrapping to zero if the value exceeds the maximum
representable value of the depthStencilAttachment ’s stencil
aspect. 

"decrement-wrap" 

Decrement the current stencil value, wrapping to the maximum representable value of the
depthStencilAttachment ’s stencil aspect if the value goes below
0 . 

10.3.7. Vertex State 

enum GPUIndexFormat {
"uint16" ,
"uint32" ,
};

The index format determines both the data type of index values in a buffer and, when used with
strip primitive topologies ( "line-strip" or
"triangle-strip" ) also specifies the primitive restart value. The
primitive restart value indicates which index value indicates that a new primitive
should be started rather than continuing to construct the triangle strip with the prior indexed
vertices. 

GPUPrimitiveState s that specify a strip primitive topology must specify a
stripIndexFormat if they are used for indexed draws
so that the primitive restart value that will be used is known at pipeline
creation time. GPUPrimitiveState s that specify a list primitive
topology will use the index format passed to setIndexBuffer() 
when doing indexed rendering. 

Index format

Byte size

Primitive restart value

"uint16" 

2

0xFFFF

"uint32" 

4

0xFFFFFFFF

10.3.7.1. Vertex Formats 

The GPUVertexFormat of a vertex attribute indicates how data from a vertex buffer will
be interpreted and exposed to the shader. The name of the format specifies the order of components,
bits per component, and vertex data type for the component. 

Each vertex data type can map to any WGSL scalar type of the same base type,
regardless of the bits per component: 

Vertex format prefix

Vertex data type

Compatible WGSL types

uint 

unsigned int

u32 

sint 

signed int

i32 

unorm 

unsigned normalized

f16 , f32 

snorm 

signed normalized

float 

floating point

The multi-component formats specify the number of components after "x". Mismatches in the number of
components between the vertex format and shader type are allowed, with components being either
dropped or filled with default values to compensate. 

A vertex attribute with a format of "unorm8x2" and byte values [0x7F, 0xFF] 
can be accessed in the shader with the following types:

Shader type

Shader value

f16 

0.5h 

f32 

0.5f 

vec2<f16> 

vec2(0.5h, 1.0h) 

vec2<f32> 

vec2(0.5f, 1.0f) 

vec3<f16> 

vec2(0.5h, 1.0h, 0.0h) 

vec3<f32> 

vec2(0.5f, 1.0f, 0.0f) 

vec4<f16> 

vec2(0.5h, 1.0h, 0.0h, 1.0h) 

vec4<f32> 

vec2(0.5f, 1.0f, 0.0f, 1.0f) 

See § 23.2.2 Vertex Processing for additional information about how vertex formats are exposed in the
shader. 

enum GPUVertexFormat {
"uint8" ,
"uint8x2" ,
"uint8x4" ,
"sint8" ,
"sint8x2" ,
"sint8x4" ,
"unorm8" ,
"unorm8x2" ,
"unorm8x4" ,
"snorm8" ,
"snorm8x2" ,
"snorm8x4" ,
"uint16" ,
"uint16x2" ,
"uint16x4" ,
"sint16" ,
"sint16x2" ,
"sint16x4" ,
"unorm16" ,
"unorm16x2" ,
"unorm16x4" ,
"snorm16" ,
"snorm16x2" ,
"snorm16x4" ,
"float16" ,
"float16x2" ,
"float16x4" ,
"float32" ,
"float32x2" ,
"float32x3" ,
"float32x4" ,
"uint32" ,
"uint32x2" ,
"uint32x3" ,
"uint32x4" ,
"sint32" ,
"sint32x2" ,
"sint32x3" ,
"sint32x4" ,
"unorm10-10-10-2" ,
"unorm8x4-bgra" ,
};

Vertex format

Data type

Components

byteSize 

Example WGSL type

"uint8" 

unsigned int

1

1

u32 

"uint8x2" 

unsigned int

2

2

vec2<u32> 

"uint8x4" 

unsigned int

4

4

vec4<u32> 

"sint8" 

signed int

1

1

i32 

"sint8x2" 

signed int

2

2

vec2<i32> 

"sint8x4" 

signed int

4

4

vec4<i32> 

"unorm8" 

unsigned normalized

1

1

f32 

"unorm8x2" 

unsigned normalized

2

2

vec2<f32> 

"unorm8x4" 

unsigned normalized

4

4

vec4<f32> 

"snorm8" 

signed normalized

1

1

f32 

"snorm8x2" 

signed normalized

2

2

vec2<f32> 

"snorm8x4" 

signed normalized

4

4

vec4<f32> 

"uint16" 

unsigned int

1

2

u32 

"uint16x2" 

unsigned int

2

4

vec2<u32> 

"uint16x4" 

unsigned int

4

8

vec4<u32> 

"sint16" 

signed int

1

2

i32 

"sint16x2" 

signed int

2

4

vec2<i32> 

"sint16x4" 

signed int

4

8

vec4<i32> 

"unorm16" 

unsigned normalized

1

2

f32 

"unorm16x2" 

unsigned normalized

2

4

vec2<f32> 

"unorm16x4" 

unsigned normalized

4

8

vec4<f32> 

"snorm16" 

signed normalized

1

2

f32 

"snorm16x2" 

signed normalized

2

4

vec2<f32> 

"snorm16x4" 

signed normalized

4

8

vec4<f32> 

"float16" 

float

1

2

f32 

"float16x2" 

float

2

4

vec2<f16> 

"float16x4" 

float

4

8

vec4<f16> 

"float32" 

float

1

4

f32 

"float32x2" 

float

2

8

vec2<f32> 

"float32x3" 

float

3

12

vec3<f32> 

"float32x4" 

float

4

16

vec4<f32> 

"uint32" 

unsigned int

1

4

u32 

"uint32x2" 

unsigned int

2

8

vec2<u32> 

"uint32x3" 

unsigned int

3

12

vec3<u32> 

"uint32x4" 

unsigned int

4

16

vec4<u32> 

"sint32" 

signed int

1

4

i32 

"sint32x2" 

signed int

2

8

vec2<i32> 

"sint32x3" 

signed int

3

12

vec3<i32> 

"sint32x4" 

signed int

4

16

vec4<i32> 

"unorm10-10-10-2" 

unsigned normalized

4

4

vec4<f32> 

"unorm8x4-bgra" 

unsigned normalized

4

4

vec4<f32> 

enum GPUVertexStepMode {
"vertex" ,
"instance" ,
};

The step mode configures how an address for vertex buffer data is computed, based on the
current vertex or instance index: 

"vertex" 

The address is advanced by arrayStride for each vertex,
and reset between instances. 

"instance" 

The address is advanced by arrayStride for each instance. 

dictionary GPUVertexState 
: GPUProgrammableStage {
sequence < GPUVertexBufferLayout ?> " href="#dom-gpuvertexstate-buffers" id="ref-for-dom-gpuvertexstate-buffers④"> buffers = [];
};

buffers , of type sequence<GPUVertexBufferLayout?> , defaulting to [] 

A list of GPUVertexBufferLayout s, each defining the layout of vertex attribute data in a
vertex buffer used by this pipeline. 

A vertex buffer is, conceptually, a view into buffer memory as an array of structures .
arrayStride is the stride, in bytes, between elements of that array.
Each element of a vertex buffer is like a structure with a memory layout defined by its
attributes , which describe the members of the structure. 

Each GPUVertexAttribute describes its
format and its
offset , in bytes, within the structure. 

Each attribute appears as a separate input in a vertex shader, each bound by a numeric location ,
which is specified by shaderLocation .
Every location must be unique within the GPUVertexState . 

dictionary GPUVertexBufferLayout {
required GPUSize64 arrayStride ;
GPUVertexStepMode stepMode = "vertex";
required sequence < GPUVertexAttribute > " href="#dom-gpuvertexbufferlayout-attributes" id="ref-for-dom-gpuvertexbufferlayout-attributes②"> attributes ;
};

arrayStride , of type GPUSize64 

The stride, in bytes, between elements of this array. 

stepMode , of type GPUVertexStepMode , defaulting to "vertex" 

Whether each element of this array represents per-vertex data or per-instance data 

attributes , of type sequence< GPUVertexAttribute > 

An array defining the layout of the vertex attributes within each element. 

dictionary GPUVertexAttribute {
required GPUVertexFormat format ;
required GPUSize64 offset ;

required GPUIndex32 shaderLocation ;
};

format , of type GPUVertexFormat 

The GPUVertexFormat of the attribute. 

offset , of type GPUSize64 

The offset, in bytes, from the beginning of the element to the data for the attribute. 

shaderLocation , of type GPUIndex32 

The numeric location associated with this attribute, which will correspond with a
"@location" attribute 
declared in the vertex . module . 

validating GPUVertexBufferLayout (device, descriptor)

Arguments: 

GPUDevice device 

GPUVertexBufferLayout descriptor 

Device timeline steps: 

Return true , if and only if, all of the following conditions are satisfied: 

descriptor . arrayStride ≤
device . [[device]] . [[limits]] . maxVertexBufferArrayStride . 

descriptor . arrayStride is a multiple of 4. 

For each attribute attrib in the list descriptor . attributes : 

If descriptor . arrayStride is zero: 

attrib . offset + byteSize ( attrib . format ) ≤
device . [[device]] . [[limits]] . maxVertexBufferArrayStride . 

Otherwise: 

attrib . offset + byteSize ( attrib . format ) ≤
descriptor . arrayStride . 

attrib . offset is a multiple of the minimum of 4 and
byteSize ( attrib . format ). 

attrib . shaderLocation is <
device . [[device]] . [[limits]] . maxVertexAttributes . 

validating GPUVertexState (device, descriptor, layout)

Arguments: 

GPUDevice device 

GPUVertexState descriptor 

GPUPipelineLayout layout 

Device timeline steps: 

Let entryPoint be get the entry point ( VERTEX , descriptor ). 

Assert entryPoint is not null . 

All of the requirements in the following steps must be met. 

validating GPUProgrammableStage ( VERTEX , descriptor , layout , device ) must succeed. 

descriptor . buffers . size must be ≤
device . [[device]] . [[limits]] . maxVertexBuffers . 

Each vertexBuffer layout descriptor in the list descriptor . buffers 
must pass validating GPUVertexBufferLayout ( device , vertexBuffer ). 

Let totalEffectiveVertexAttributes be the sum of vertexBuffer . attributes . size ,
over every vertexBuffer in descriptor . buffers . 

If device . [[features]] does not contain "core-features-and-limits" :

If the vertex_index builtin is a shader stage input of
descriptor . vertex : 

Add 1 to totalEffectiveVertexAttributes 

If the instance_index builtin is a shader stage input of
descriptor . vertex : 

Add 1 to totalEffectiveVertexAttributes 

totalEffectiveVertexAttributes must be ≤
device . [[device]] . [[limits]] . maxVertexAttributes . 

For every vertex attribute declaration (at location location with type T ) that is
statically used by entryPoint , there must be exactly one pair ( i , j ) for which
descriptor . buffers [ i ]?. attributes [ j ]. shaderLocation == location . 

Let attrib be that GPUVertexAttribute . 

T must be compatible with attrib . format ’s vertex data type : 

"unorm", "snorm", or "float"

T must be f32 or vecN<f32> . 

"uint"

T must be u32 or vecN<u32> . 

"sint"

T must be i32 or vecN<i32> . 

11. Copies 

11.1. Buffer Copies 

Buffer copy operations operate on raw bytes. 

WebGPU provides "buffered" GPUCommandEncoder commands: 

copyBufferToBuffer() 

clearBuffer() 

and "immediate" GPUQueue operations: 

writeBuffer() , for ArrayBuffer -to- GPUBuffer writes 

11.2. Texel Copies

Texel copy operations operate on texture/"image" data, rather than bytes. 

WebGPU provides "buffered" GPUCommandEncoder commands: 

copyTextureToTexture() 

copyBufferToTexture() 

copyTextureToBuffer() 

and "immediate" GPUQueue operations: 

writeTexture() , for ArrayBuffer -to- GPUTexture writes 

copyExternalImageToTexture() , for copies from Web Platform image sources to textures 

In a texel copy, the bytes written to the destination texel blocks will have an
equivalent texel representation to the source value.

Texel copies only guarantee that valid, finite, non-subnormal numeric values
in the source have the same numeric value in the destination.
Specifically, the texel block may be decoded and re-encoded in a way that
preserves only those values.
Where multiple byte representations are possible, the choice of representation is implementation-defined. 

Any floating-point zero value may be represented as either -0.0 or +0.0. 

Any floating-point subnormal value may be either preserved or replaced by -0.0 or +0.0. 

Any floating-point NaN or Infinity value may be replaced by an indeterminate value . 

Packed formats and snorm formats may change bit-representation as long
as the represented values follow the rules above, for example: 

snorm formats may represent -1.0 as either -127 or -128. 

Formats like "rgb9e5ufloat" have multiple bit-representations of some values. 

Note: 
For formats supporting RENDER_ATTACHMENT or
STORAGE_BINDING , this can be thought of as similar to,
and may be implemented as, writing the texture using a WGSL shader.
In general, any WGSL floating point behaviors may be observed. 

The following definitions are used by these methods: 

11.2.1. GPUTexelCopyBufferLayout 

" GPUTexelCopyBufferLayout " describes the " layout " of texels in a " buffer " of bytes
( GPUBuffer or AllowSharedBufferSource ) in a " texel copy " operation. 

dictionary GPUTexelCopyBufferLayout {
GPUSize64 offset = 0;
GPUSize32 bytesPerRow ;
GPUSize32 rowsPerImage ;
};

A texel image is comprised of one or more rows of texel blocks , referred to here
as texel block row s. Each texel block row of a texel image must contain the
same number of texel blocks , and all texel blocks in a texel image are of the same
GPUTextureFormat . 

A GPUTexelCopyBufferLayout is a layout of texel images within some linear memory.
It’s used when copying data between a texture and a GPUBuffer , or when scheduling a
write into a texture from the GPUQueue . 

For 2d textures, data is copied between one or multiple contiguous
texel images and array layers . 

For 3d textures, data is copied between one or multiple contiguous
texel images and depth slices . 

Operations that copy between byte arrays and textures always operate on whole texel block .
It’s not possible to update only a part of a texel block . 

Texel blocks are tightly packed within each texel block row in the linear memory layout of a
texel copy , with each subsequent texel block immediately following the previous texel block ,
with no padding.
This includes copies to/from specific aspects of depth-or-stencil format textures:
stencil values are tightly packed in an array of bytes;
depth values are tightly packed in an array of the appropriate type ("depth16unorm" or "depth32float"). 

offset , of type GPUSize64 , defaulting to 0 

The offset, in bytes, from the beginning of the texel data source (such as a
GPUTexelCopyBufferInfo.buffer ) to the start of the texel data
within that source. 

bytesPerRow , of type GPUSize32 

The stride, in bytes, between the beginning of each texel block row and the subsequent
texel block row . 

Required if there are multiple texel block rows (i.e. the copy height or depth is more
than one block). 

rowsPerImage , of type GPUSize32 

Number of texel block rows per single texel image of the texture .
rowsPerImage ×
bytesPerRow is the stride, in bytes, between the beginning of each
texel image of data and the subsequent texel image . 

Required if there are multiple texel images (i.e. the copy depth is more than one). 

11.2.2. GPUTexelCopyBufferInfo 

" GPUTexelCopyBufferInfo " describes the " info " ( GPUBuffer and GPUTexelCopyBufferLayout )
about a " buffer " source or destination of a " texel copy " operation.
Together with the copySize , it describes the footprint of a region of texels in a GPUBuffer . 

dictionary GPUTexelCopyBufferInfo 
: GPUTexelCopyBufferLayout {
required GPUBuffer buffer ;
};

buffer , of type GPUBuffer 

A buffer which either contains texel data to be copied or will store the texel data being
copied, depending on the method it is being passed to. 

validating GPUTexelCopyBufferInfo 

Arguments: 

GPUTexelCopyBufferInfo imageCopyBuffer 

Returns: boolean 

Device timeline steps: 

Return true if and only if all of the following conditions are satisfied: 

imageCopyBuffer . buffer must be a valid GPUBuffer . 

imageCopyBuffer . bytesPerRow must be a multiple of 256. 

11.2.3. GPUTexelCopyTextureInfo 

" GPUTexelCopyTextureInfo " describes the " info " ( GPUTexture , etc.)
about a " texture " source or destination of a " texel copy " operation.
Together with the copySize , it describes a sub-region of a texture
(spanning one or more contiguous texture subresources at the same mip-map level). 

dictionary GPUTexelCopyTextureInfo {
required GPUTexture texture ;
GPUIntegerCoordinate mipLevel = 0;
GPUOrigin3D origin = {};
GPUTextureAspect aspect = "all";
};

texture , of type GPUTexture 

Texture to copy to/from. 

mipLevel , of type GPUIntegerCoordinate , defaulting to 0 

Mip-map level of the texture to copy to/from. 

origin , of type GPUOrigin3D , defaulting to {} 

Defines the origin of the copy - the minimum corner of the texture sub-region to copy to/from.
Together with copySize , defines the full copy sub-region. 

aspect , of type GPUTextureAspect , defaulting to "all" 

Defines which aspects of the texture to copy to/from. 

The texture copy sub-region for depth slice or array layer index of GPUTexelCopyTextureInfo 
copyTexture is determined by running the following steps:

Let texture be copyTexture . texture . 

If texture . dimension is: 

1d 

Assert index is 0 

Let depthSliceOrLayer be texture 

2d 

Let depthSliceOrLayer be array layer index of texture 

3d 

Let depthSliceOrLayer be depth slice index of texture 

Let textureMip be mip level copyTexture . mipLevel of depthSliceOrLayer . 

Return aspect copyTexture . aspect of textureMip . 

The texel block byte offset of data described by GPUTexelCopyBufferLayout bufferLayout 
corresponding to texel block x , y of depth slice or array layer z of a GPUTexture texture is
determined by running the following steps:

Let blockBytes be the texel block copy footprint of texture . format . 

Let imageOffset be ( z × bufferLayout . rowsPerImage ×
bufferLayout . bytesPerRow ) + bufferLayout . offset . 

Let rowOffset be ( y × bufferLayout . bytesPerRow ) + imageOffset . 

Let blockOffset be ( x × blockBytes ) + rowOffset . 

Return blockOffset . 

validating GPUTexelCopyTextureInfo ( texelCopyTextureInfo , copySize )

Arguments: 

GPUTexelCopyTextureInfo texelCopyTextureInfo 

GPUExtent3D copySize 

Returns: boolean 

Device timeline steps: 

Let blockWidth be the texel block width of texelCopyTextureInfo . texture . format . 

Let blockHeight be the texel block height of texelCopyTextureInfo . texture . format . 

Return true if and only if all of the following conditions apply: 

validating texture copy range ( texelCopyTextureInfo , copySize ) returns true . 

texelCopyTextureInfo . texture must be a valid GPUTexture . 

texelCopyTextureInfo . mipLevel must be <
texelCopyTextureInfo . texture . mipLevelCount . 

texelCopyTextureInfo . origin . x must be a multiple of blockWidth . 

texelCopyTextureInfo . origin . y must be a multiple of blockHeight . 

The GPUTexelCopyTextureInfo physical subresource size of texelCopyTextureInfo is equal to copySize if either of
the following conditions is true: 

texelCopyTextureInfo . texture . format is a depth-stencil format. 

texelCopyTextureInfo . texture . sampleCount > 1. 

validating texture buffer copy ( texelCopyTextureInfo , bufferLayout , dataLength , copySize , textureUsage , aligned )

Arguments: 

GPUTexelCopyTextureInfo texelCopyTextureInfo 

GPUTexelCopyBufferLayout bufferLayout 

GPUSize64Out dataLength 

GPUExtent3D copySize 

GPUTextureUsage textureUsage 

boolean aligned 

Returns: boolean 

Device timeline steps: 

Let texture be texelCopyTextureInfo . texture 

Let aspectSpecificFormat = texture . format . 

Let offsetAlignment = texel block copy footprint of texture . format . 

Return true if and only if all of the following conditions apply: 

validating GPUTexelCopyTextureInfo ( texelCopyTextureInfo , copySize ) returns true . 

texture . sampleCount is 1. 

texture . usage contains textureUsage . 

If texture . format is a depth-or-stencil format format: 

texelCopyTextureInfo . aspect must refer to a single aspect of
texture . format . 

If textureUsage is: 

COPY_SRC 

That aspect must be a valid texel copy source according to § 26.1.2 Depth-stencil formats . 

COPY_DST 

That aspect must be a valid texel copy destination according to § 26.1.2 Depth-stencil formats . 

Set aspectSpecificFormat to the aspect-specific format according to § 26.1.2 Depth-stencil formats . 

Set offsetAlignment to 4. 

If aligned is true : 

bufferLayout . offset is a multiple of offsetAlignment . 

validating linear texture data ( bufferLayout ,
dataLength ,
aspectSpecificFormat ,
copySize ) succeeds. 

11.2.4. GPUCopyExternalImageDestInfo 

WebGPU textures hold raw numeric data, and are not tagged with semantic metadata describing colors.
However, copyExternalImageToTexture() copies from sources that describe colors. 

" GPUCopyExternalImageDestInfo " describes the " info " about the " dest ination" of a
" copyExternalImage ToTexture() " operation.
It is a GPUTexelCopyTextureInfo which is additionally tagged with
color space/encoding and alpha-premultiplication metadata, so that semantic color data may be
preserved during copies.
This metadata affects only the semantics of the copy operation
operation, not the state or semantics of the destination texture object. 

dictionary GPUCopyExternalImageDestInfo 
: GPUTexelCopyTextureInfo {
PredefinedColorSpace colorSpace = "srgb";
boolean premultipliedAlpha = false ;
};

colorSpace , of type PredefinedColorSpace , defaulting to "srgb" 

Describes the color space and encoding used to encode data into the destination texture. 

This may result in values outside of the range [0, 1]
being written to the target texture, if its format can represent them.
Otherwise, the results are clamped to the target texture format’s range. 

Note: 
If colorSpace matches the source image,
conversion might not be necessary. See § 3.11.2 Color Space Conversion Elision . 

premultipliedAlpha , of type boolean , defaulting to false 

Describes whether the data written into the texture should have its RGB channels
premultiplied by the alpha channel, or not. 

If this option is set to true and the source is also
premultiplied, the source RGB values must be preserved even if they exceed their
corresponding alpha values. 

Note: 
If premultipliedAlpha matches the source image,
conversion might not be necessary. See § 3.11.2 Color Space Conversion Elision . 

11.2.5. GPUCopyExternalImageSourceInfo 

" GPUCopyExternalImageSourceInfo " describes the " info " about the " source " of a
" copyExternalImage ToTexture() " operation. 

typedef ( ImageBitmap or 
ImageData or 
HTMLImageElement or 
HTMLVideoElement or 
VideoFrame or 
HTMLCanvasElement or 
OffscreenCanvas ) GPUCopyExternalImageSource ;

dictionary GPUCopyExternalImageSourceInfo {
required GPUCopyExternalImageSource source ;
GPUOrigin2D origin = {};
boolean flipY = false ;
};

GPUCopyExternalImageSourceInfo has the following members: 

source , of type GPUCopyExternalImageSource 

The source of the texel copy . The copy source data is captured at the moment that
copyExternalImageToTexture() is issued. Source size is determined as described
by the external source dimensions table. 

origin , of type GPUOrigin2D , defaulting to {} 

Defines the origin of the copy - the minimum (top-left) corner of the source sub-region to copy from.
Together with copySize , defines the full copy sub-region. 

flipY , of type boolean , defaulting to false 

Describes whether the source image is vertically flipped, or not. 

If this option is set to true , the copy is flipped vertically: the bottom row of the source
region is copied into the first row of the destination region, and so on.
The origin option is still relative to the top-left corner
of the source image, increasing downward. 

When external sources are used when creating or copying to textures, the external source dimensions 
are defined by the source type, given by this table: 

External Source type

Dimensions

ImageBitmap 

ImageBitmap.width ,
ImageBitmap.height 

HTMLImageElement 

HTMLImageElement.naturalWidth ,
HTMLImageElement.naturalHeight 

HTMLVideoElement 

intrinsic width of the frame ,
intrinsic height of the frame 

VideoFrame 

VideoFrame.displayWidth ,
VideoFrame.displayHeight 

ImageData 

ImageData.width ,
ImageData.height 

HTMLCanvasElement or OffscreenCanvas with CanvasRenderingContext2D or GPUCanvasContext 

HTMLCanvasElement.width ,
HTMLCanvasElement.height 

HTMLCanvasElement or OffscreenCanvas with WebGLRenderingContextBase 

WebGLRenderingContextBase.drawingBufferWidth ,
WebGLRenderingContextBase.drawingBufferHeight 

HTMLCanvasElement or OffscreenCanvas with ImageBitmapRenderingContext 

ImageBitmapRenderingContext ’s internal output bitmap
ImageBitmap.width ,
ImageBitmap.height 

11.2.6. Subroutines 

GPUTexelCopyTextureInfo physical subresource size 

Arguments: 

GPUTexelCopyTextureInfo texelCopyTextureInfo 

Returns: GPUExtent3D 

The GPUTexelCopyTextureInfo physical subresource size of texelCopyTextureInfo is calculated as follows: 

Its width , height and depthOrArrayLayers are the width, height, and depth, respectively,
of the physical miplevel-specific texture extent of texelCopyTextureInfo . texture subresource at mipmap level 
texelCopyTextureInfo . mipLevel . 

validating linear texture data (layout, byteSize, format, copyExtent)

Arguments: 

GPUTexelCopyBufferLayout layout 

Layout of the linear texture data. 

GPUSize64 byteSize 

Total size of the linear data, in bytes. 

GPUTextureFormat format 

Format of the texture. 

GPUExtent3D copyExtent 

Extent of the texture to copy. 

Device timeline steps: 

Let: 

widthInBlocks be copyExtent . width ÷ the texel block width of format .
Assert this is an integer. 

heightInBlocks be copyExtent . height ÷ the texel block height of format .
Assert this is an integer. 

bytesInLastRow be widthInBlocks × the texel block copy footprint of format . 

Fail if the following input validation requirements are not met: 

If heightInBlocks > 1,
layout . bytesPerRow must be specified. 

If copyExtent . depthOrArrayLayers > 1,
layout . bytesPerRow and
layout . rowsPerImage must be specified. 

If specified, layout . bytesPerRow 
must be ≥ bytesInLastRow . 

If specified, layout . rowsPerImage 
must be ≥ heightInBlocks . 

Let: 

bytesPerRow be layout . bytesPerRow ?? 0. 

rowsPerImage be layout . rowsPerImage ?? 0. 

Note: These default values have no effect, as they’re always multiplied by 0. 

Let requiredBytesInCopy be 0. 

If copyExtent . depthOrArrayLayers > 0: 

Increment requiredBytesInCopy by
bytesPerRow × rowsPerImage × ( copyExtent . depthOrArrayLayers − 1). 

If heightInBlocks > 0: 

Increment requiredBytesInCopy by
bytesPerRow × ( heightInBlocks − 1) + bytesInLastRow . 

Fail if the following condition is not satisfied: 

The layout fits inside the linear data:
layout . offset + requiredBytesInCopy ≤ byteSize . 

validating texture copy range 

Arguments: 

GPUTexelCopyTextureInfo texelCopyTextureInfo 

The texture subresource being copied into and copy origin. 

GPUExtent3D copySize 

The size of the texture. 

Device timeline steps: 

Let blockWidth be the texel block width of texelCopyTextureInfo . texture . format . 

Let blockHeight be the texel block height of texelCopyTextureInfo . texture . format . 

Let subresourceSize be the GPUTexelCopyTextureInfo physical subresource size of texelCopyTextureInfo . 

Return whether all the conditions below are satisfied: 

( texelCopyTextureInfo . origin . x + copySize . width ) ≤ subresourceSize . width 

( texelCopyTextureInfo . origin . y + copySize . height ) ≤ subresourceSize . height 

( texelCopyTextureInfo . origin . z + copySize . depthOrArrayLayers ) ≤ subresourceSize . depthOrArrayLayers 

copySize . width must be a multiple of blockWidth . 

copySize . height must be a multiple of blockHeight . 

Note: 
The texture copy range is validated against the physical (rounded-up)
size for compressed formats , allowing copies to access texture
blocks which are not fully inside the texture. 

Two GPUTextureFormat s format1 and format2 are copy-compatible if:

format1 equals format2 , or 

format1 and format2 differ only in whether they are srgb formats (have the -srgb suffix). 

The set of subresources for texture copy ( texelCopyTextureInfo , copySize )
is the subset of subresources of texture = texelCopyTextureInfo . texture 
for which each subresource s satisfies the following:

The mipmap level of s equals
texelCopyTextureInfo . mipLevel . 

The aspect of s is in the set of aspects of
texelCopyTextureInfo . aspect . 

If texture . dimension is "2d" : 

The array layer of s is ≥
texelCopyTextureInfo . origin . z and <
texelCopyTextureInfo . origin . z +
copySize . depthOrArrayLayers . 

12. Command Buffers 

Command buffers are pre-recorded lists of GPU commands (blocks of queue timeline 
steps) that can be submitted to a GPUQueue for execution.
Each GPU command represents a task to be performed on the
queue timeline , such as setting state, drawing, copying resources, etc. 

A GPUCommandBuffer can only be submitted once, at which point it becomes invalidated .
To reuse rendering commands across multiple submissions, use GPURenderBundle . 

12.1. GPUCommandBuffer 

[ Exposed =( Window , Worker ), SecureContext ]
interface GPUCommandBuffer {
};
GPUCommandBuffer includes GPUObjectBase ;

GPUCommandBuffer has the following device timeline properties : 

[[command_list]] , of type list < GPU command >, readonly

A list of GPU commands to be executed on the Queue timeline when this command
buffer is submitted. 

[[renderState]] , of type RenderState , initially null 

The current state used by any render pass commands being executed. 

[[used_bind_groups]] , of type set < GPUBindGroup >, readonly

A set of all GPUBindGroup s used by this command buffer. 

12.1.1. Command Buffer Creation 

dictionary GPUCommandBufferDescriptor 
: GPUObjectDescriptorBase {
};

13. Command Encoding 

13.1. GPUCommandsMixin 

GPUCommandsMixin defines state common to all interfaces which encode commands.
It has no methods. 

interface mixin GPUCommandsMixin {
};

GPUCommandsMixin has the following device timeline properties : 

[[state]] , of type encoder state , initially " open "

The current state of the encoder. 

[[commands]] , of type list < GPU command >, initially [] 

A list of GPU commands to be executed on the Queue timeline when a
GPUCommandBuffer containing these commands is submitted. 

[[used_bind_groups]] , of type set < GPUBindGroup >, initially empty;

A set of all GPUBindGroup s set with setBindGroup() during command encoding. 

The encoder state may be one of the following: 

" open "

The encoder is available to encode new commands. 

" locked "

The encoder cannot be used, because it is locked by a child encoder: it is a
GPUCommandEncoder , and a GPURenderPassEncoder or GPUComputePassEncoder is active.
The encoder becomes " open " again when the pass is ended. 

Any command issued in this state invalidates the encoder. 

" ended "

The encoder has been ended and new commands can no longer be encoded. 

Any command issued in this state will generate a validation error . 

To Validate the encoder state of GPUCommandsMixin encoder run the 

following device timeline steps:

If encoder . [[state]] is: 

" open "

Return true . 

" locked "

Invalidate encoder and return false . 

" ended "

Generate a validation error , and return false . 

To Enqueue a command on GPUCommandsMixin encoder 
which issues the steps of a GPU Command command , run the following device timeline steps:

Append command to encoder . [[commands]] . 

When command is executed as part of a GPUCommandBuffer : 

Issue the steps of command . 

13.2. GPUCommandEncoder 

[ Exposed =( Window , Worker ), SecureContext ]
interface GPUCommandEncoder {
GPURenderPassEncoder beginRenderPass ( GPURenderPassDescriptor descriptor );
GPUComputePassEncoder beginComputePass ( optional GPUComputePassDescriptor descriptor = {});

undefined copyBufferToBuffer (
GPUBuffer source ,
GPUBuffer destination ,
optional GPUSize64 size );
undefined copyBufferToBuffer (
GPUBuffer source ,
GPUSize64 sourceOffset ,
GPUBuffer destination ,
GPUSize64 destinationOffset ,
optional GPUSize64 size );

undefined copyBufferToTexture (
GPUTexelCopyBufferInfo source ,
GPUTexelCopyTextureInfo destination ,
GPUExtent3D copySize );

undefined copyTextureToBuffer (
GPUTexelCopyTextureInfo source ,
GPUTexelCopyBufferInfo destination ,
GPUExtent3D copySize );

undefined copyTextureToTexture (
GPUTexelCopyTextureInfo source ,
GPUTexelCopyTextureInfo destination ,
GPUExtent3D copySize );

undefined clearBuffer (
GPUBuffer buffer ,
optional GPUSize64 offset = 0,
optional GPUSize64 size );

undefined resolveQuerySet (
GPUQuerySet querySet ,
GPUSize32 firstQuery ,
GPUSize32 queryCount ,
GPUBuffer destination ,
GPUSize64 destinationOffset );

GPUCommandBuffer finish ( optional GPUCommandBufferDescriptor descriptor = {});
};
GPUCommandEncoder includes GPUObjectBase ;
GPUCommandEncoder includes GPUCommandsMixin ;
GPUCommandEncoder includes GPUDebugCommandsMixin ;

13.2.1. Command Encoder Creation 

dictionary GPUCommandEncoderDescriptor 
: GPUObjectDescriptorBase {
};

createCommandEncoder(descriptor) 

Creates a GPUCommandEncoder . 

Called on: GPUDevice this.

Arguments: 

Arguments for the GPUDevice.createCommandEncoder(descriptor) method. 

Parameter
Type
Nullable
Optional
Description

descriptor 
GPUCommandEncoderDescriptor 
✘ 
✔ 
Description of the GPUCommandEncoder to create.

Returns: GPUCommandEncoder 

Content timeline steps: 

Let e be ! create a new WebGPU object ( this , GPUCommandEncoder , descriptor ). 

Issue the initialization steps on the Device timeline of this . 

Return e . 

Device timeline initialization steps :

If any of the following conditions are unsatisfied
generate a validation error , invalidate e and return. 

this must not be lost . 

Creating a GPUCommandEncoder , encoding a command to clear a buffer, finishing the
encoder to get a GPUCommandBuffer , then submitting it to the GPUQueue .

const commandEncoder = gpuDevice . createCommandEncoder (); 
commandEncoder . clearBuffer ( buffer ); 
const commandBuffer = commandEncoder . finish (); 
gpuDevice . queue . submit ([ commandBuffer ]); 

13.3. Pass Encoding 

beginRenderPass(descriptor) 

Begins encoding a render pass described by descriptor . 

Called on: GPUCommandEncoder this .

Arguments: 

Arguments for the GPUCommandEncoder.beginRenderPass(descriptor) method. 

Parameter
Type
Nullable
Optional
Description

descriptor 
GPURenderPassDescriptor 
✘ 
✘ 
Description of the GPURenderPassEncoder to create.

Returns: GPURenderPassEncoder 

Content timeline steps: 

For each non- null colorAttachment in descriptor . colorAttachments : 

If colorAttachment . clearValue is provided : 

? validate GPUColor shape ( colorAttachment . clearValue ). 

Let pass be a new GPURenderPassEncoder object. 

Issue the initialization steps on the Device timeline of this . 

Return pass . 

Device timeline initialization steps :

Validate the encoder state of this .
If it returns false, invalidate pass and return. 

Set this . [[state]] to " locked ". 

Let attachmentRegions be a list of [ texture subresource , depthSlice ?]
pairs, initially empty. Each pair describes the region of the texture to be rendered to, which
includes a single depth slice for "3d" textures only. 

For each non- null colorAttachment in descriptor . colorAttachments : 

Add [ colorAttachment . view ,
colorAttachment . depthSlice ?? null ] to attachmentRegions . 

If colorAttachment . resolveTarget is not null : 

Add [ colorAttachment . resolveTarget ,
undefined ] to attachmentRegions . 

If any of the following requirements are unmet, invalidate pass and return. 

descriptor must meet the Valid Usage rules
given device this . [[device]] . 

The set of texture regions in attachmentRegions must be pairwise disjoint.
That is, no two texture regions may overlap. 

Add each texture subresource in attachmentRegions 
to pass . [[usage scope]] 
with usage attachment . 

Let depthStencilAttachment be descriptor . depthStencilAttachment . 

If depthStencilAttachment is not null : 

Let depthStencilView be depthStencilAttachment . view . 

Add the depth subresource of depthStencilView , if any,
to pass . [[usage scope]] 
with usage attachment-read if
depthStencilAttachment . depthReadOnly is true,
or attachment otherwise. 

Add the stencil subresource of depthStencilView , if any,
to pass . [[usage scope]] 
with usage attachment-read if
depthStencilAttachment . stencilReadOnly is true,
or attachment otherwise. 

Set pass . [[depthReadOnly]] to depthStencilAttachment . depthReadOnly . 

Set pass . [[stencilReadOnly]] to depthStencilAttachment . stencilReadOnly . 

Set pass . [[layout]] to derive render targets layout from pass ( descriptor ). 

If descriptor . timestampWrites is provided : 

Let timestampWrites be descriptor . timestampWrites . 

If timestampWrites . beginningOfPassWriteIndex 
is provided ,
append a GPU command to this . [[commands]] 
with the following steps: 

Before the pass commands begin executing,
write the current queue timestamp into index
timestampWrites . beginningOfPassWriteIndex 
of timestampWrites . querySet . 

If timestampWrites . endOfPassWriteIndex 
is provided , set pass . [[endTimestampWrite]] 
to a GPU command with the following steps: 

After the pass commands finish executing,
write the current queue timestamp into index
timestampWrites . endOfPassWriteIndex 
of timestampWrites . querySet . 

Set pass . [[drawCount]] to 0. 

Set pass . [[maxDrawCount]] to descriptor . maxDrawCount . 

Set pass . [[maxDrawCount]] to descriptor . maxDrawCount . 

Enqueue a command on this which issues the subsequent steps on the
Queue timeline when executed. 

Queue timeline steps:

Let the [[renderState]] of the currently executing
GPUCommandBuffer be a new RenderState . 

Set [[renderState]] . [[colorAttachments]] to
descriptor . colorAttachments . 

Set [[renderState]] . [[depthStencilAttachment]] to
descriptor . depthStencilAttachment . 

For each non- null colorAttachment in descriptor . colorAttachments : 

Let colorView be colorAttachment . view . 

If colorView . [[descriptor]] . dimension is: 

"3d" 

Let colorSubregion be colorAttachment . depthSlice of
colorView . 

Otherwise

Let colorSubregion be colorView . 

If colorAttachment . loadOp is: 

"load" 

Ensure the contents of colorSubregion are loaded into the framebuffer memory 
associated with colorSubregion . 

"clear" 

Set every texel of the framebuffer memory associated with
colorSubregion to colorAttachment . clearValue . 

If depthStencilAttachment is not null : 

If depthStencilAttachment . depthLoadOp is: 

Not provided 

Assert that depthStencilAttachment . depthReadOnly 
is true and ensure the contents of the depth subresource 
of depthStencilView are loaded into the framebuffer memory associated with
depthStencilView . 

"load" 

Ensure the contents of the depth subresource of
depthStencilView are loaded into the framebuffer memory associated with
depthStencilView . 

"clear" 

Set every texel of the framebuffer memory associated with the
depth subresource of depthStencilView to
depthStencilAttachment . depthClearValue . 

If depthStencilAttachment . stencilLoadOp is: 

Not provided 

Assert that depthStencilAttachment . stencilReadOnly 
is true and ensure the contents of the stencil subresource 
of depthStencilView are loaded into the framebuffer memory associated with
depthStencilView . 

"load" 

Ensure the contents of the stencil subresource of
depthStencilView are loaded into the framebuffer memory associated with
depthStencilView . 

"clear" 

Set every texel of the framebuffer memory associated with the
stencil subresource depthStencilView to
depthStencilAttachment . stencilClearValue . 

Note: Read-only depth-stencil attachments are implicitly treated as though the "load" 
operation was used. Validation that requires the load op to not be provided for read-only attachments
is done in GPURenderPassDepthStencilAttachment Valid Usage . 

beginComputePass(descriptor) 

Begins encoding a compute pass described by descriptor . 

Called on: GPUCommandEncoder this .

Arguments: 

Arguments for the GPUCommandEncoder.beginComputePass(descriptor) method. 

Parameter
Type
Nullable
Optional
Description

descriptor 
GPUComputePassDescriptor 
✘ 
✔ 

Returns: GPUComputePassEncoder 

Content timeline steps: 

Let pass be a new GPUComputePassEncoder object. 

Issue the initialization steps on the Device timeline of this . 

Return pass . 

Device timeline initialization steps :

Validate the encoder state of this .
If it returns false, invalidate pass and return. 

Set this . [[state]] to " locked ". 

If any of the following requirements are unmet, invalidate pass and return. 

If descriptor . timestampWrites is provided : 

Validate timestampWrites ( this . [[device]] ,
descriptor . timestampWrites )
must return true. 

If descriptor . timestampWrites is provided : 

Let timestampWrites be descriptor . timestampWrites . 

If timestampWrites . beginningOfPassWriteIndex 
is provided ,
append a GPU command to this . [[commands]] 
with the following steps: 

Before the pass commands begin executing,
write the current queue timestamp into index
timestampWrites . beginningOfPassWriteIndex 
of timestampWrites . querySet . 

If timestampWrites . endOfPassWriteIndex 
is provided , set pass . [[endTimestampWrite]] 
to a GPU command with the following steps: 

After the pass commands finish executing,
write the current queue timestamp into index
timestampWrites . endOfPassWriteIndex 
of timestampWrites . querySet . 

13.4. Buffer Copy Commands 

copyBufferToBuffer() has two overloads: 

copyBufferToBuffer(source, destination, size) 

Shorthand, equivalent to copyBufferToBuffer(source, 0, destination, 0, size) . 

copyBufferToBuffer(source, sourceOffset, destination, destinationOffset, size) 

Encode a command into the GPUCommandEncoder that copies data from a sub-region of a
GPUBuffer to a sub-region of another GPUBuffer . 

Called on: GPUCommandEncoder this .

Arguments: 

Arguments for the GPUCommandEncoder.copyBufferToBuffer(source, sourceOffset, destination, destinationOffset, size) method. 

Parameter
Type
Nullable
Optional
Description

source 
GPUBuffer 
✘ 
✘ 
The GPUBuffer to copy from.

sourceOffset 
GPUSize64 
✘ 
✘ 
Offset in bytes into source to begin copying from.

destination 
GPUBuffer 
✘ 
✘ 
The GPUBuffer to copy to.

destinationOffset 
GPUSize64 
✘ 
✘ 
Offset in bytes into destination to place the copied data.

size 
GPUSize64 
✘ 
✔ 
Bytes to copy.

Returns: undefined 

Content timeline steps: 

Issue the subsequent steps on the Device timeline of
this . [[device]] . 

Device timeline steps:

Validate the encoder state of this . If it returns false, return. 

If size is undefined , set it to source . size − sourceOffset . 

If any of the following conditions are unsatisfied, invalidate this and return. 

source is valid to use with this . 

destination is valid to use with this . 

source . usage contains COPY_SRC . 

destination . usage contains COPY_DST . 

size is a multiple of 4. 

sourceOffset is a multiple of 4. 

destinationOffset is a multiple of 4. 

source . size ≥ ( sourceOffset + size ). 

destination . size ≥ ( destinationOffset + size ). 

source and destination are not the same GPUBuffer . 

Enqueue a command on this which issues the subsequent steps on the
Queue timeline when executed. 

Queue timeline steps:

Copy size bytes of source , beginning at sourceOffset , into destination ,
beginning at destinationOffset . 

clearBuffer(buffer, offset, size) 

Encode a command into the GPUCommandEncoder that fills a sub-region of a
GPUBuffer with zeros. 

Called on: GPUCommandEncoder this .

Arguments: 

Arguments for the GPUCommandEncoder.clearBuffer(buffer, offset, size) method. 

Parameter
Type
Nullable
Optional
Description

buffer 
GPUBuffer 
✘ 
✘ 
The GPUBuffer to clear.

offset 
GPUSize64 
✘ 
✔ 
Offset in bytes into buffer where the sub-region to clear begins.

size 
GPUSize64 
✘ 
✔ 
Size in bytes of the sub-region to clear. Defaults to the size of the buffer minus offset .

Returns: undefined 

Content timeline steps: 

Issue the subsequent steps on the Device timeline of
this . [[device]] . 

Device timeline steps:

Validate the encoder state of this . If it returns false, return. 

If size is missing, set size to max(0, buffer . size - offset ) . 

If any of the following conditions are unsatisfied, invalidate this and return. 

buffer is valid to use with this . 

buffer . usage contains COPY_DST . 

size is a multiple of 4. 

offset is a multiple of 4. 

buffer . size ≥ ( offset + size ). 

Enqueue a command on this which issues the subsequent steps on the
Queue timeline when executed. 

Queue timeline steps:

Set size bytes of buffer to 0 starting at offset . 

13.5. Texel Copy Commands

copyBufferToTexture(source, destination, copySize) 

Encode a command into the GPUCommandEncoder that copies data from a sub-region of a
GPUBuffer to a sub-region of one or multiple continuous texture subresources . 

Called on: GPUCommandEncoder this .

Arguments: 

Arguments for the GPUCommandEncoder.copyBufferToTexture(source, destination, copySize) method. 

Parameter
Type
Nullable
Optional
Description

source 
GPUTexelCopyBufferInfo 
✘ 
✘ 
Combined with copySize , defines the region of the source buffer.

destination 
GPUTexelCopyTextureInfo 
✘ 
✘ 
Combined with copySize , defines the region of the destination texture subresource .

copySize 
GPUExtent3D 
✘ 
✘ 

Returns: undefined 

Content timeline steps: 

? validate GPUOrigin3D shape ( destination . origin ). 

? validate GPUExtent3D shape ( copySize ). 

Issue the subsequent steps on the Device timeline of this . [[device]] : 

Device timeline steps:

Validate the encoder state of this . If it returns false, return. 

Let aligned be true . 

Let dataLength be source . buffer . size . 

If any of the following conditions are unsatisfied, invalidate this and return. 

validating GPUTexelCopyBufferInfo ( source ) returns true . 

source . buffer . usage contains
COPY_SRC . 

validating texture buffer copy ( destination , source , dataLength , copySize , COPY_DST , aligned ) returns true . 

Enqueue a command on this which issues the subsequent steps on the
Queue timeline when executed. 

Queue timeline steps:

Let blockWidth be the texel block width of destination . texture . 

Let blockHeight be the texel block height of destination . texture . 

Let dstOrigin be destination . origin . 

Let dstBlockOriginX be ( dstOrigin . x ÷ blockWidth ). 

Let dstBlockOriginY be ( dstOrigin . y ÷ blockHeight ). 

Let blockColumns be ( copySize . width ÷ blockWidth ). 

Let blockRows be ( copySize . height ÷ blockHeight ). 

Assert that dstBlockOriginX , dstBlockOriginY , blockColumns , and blockRows are integers. 

For each z in the range [0, copySize . depthOrArrayLayers − 1]: 

Let dstSubregion be texture copy sub-region ( z + dstOrigin . z ) of destination . 

For each y in the range [0, blockRows − 1]: 

For each x in the range [0, blockColumns − 1]: 

Let blockOffset be the texel block byte offset of source for ( x , y , z ) of
destination . texture . 

Set texel block ( dstBlockOriginX + x , dstBlockOriginY + y ) of
dstSubregion to be an equivalent texel representation to the texel block 
described by source . buffer at offset blockOffset . 

copyTextureToBuffer(source, destination, copySize) 

Encode a command into the GPUCommandEncoder that copies data from a sub-region of one or
multiple continuous texture subresources to a sub-region of a GPUBuffer . 

Called on: GPUCommandEncoder this .

Arguments: 

Arguments for the GPUCommandEncoder.copyTextureToBuffer(source, destination, copySize) method. 

Parameter
Type
Nullable
Optional
Description

source 
GPUTexelCopyTextureInfo 
✘ 
✘ 
Combined with copySize , defines the region of the source texture subresources .

destination 
GPUTexelCopyBufferInfo 
✘ 
✘ 
Combined with copySize , defines the region of the destination buffer.

copySize 
GPUExtent3D 
✘ 
✘ 

Returns: undefined 

Content timeline steps: 

? validate GPUOrigin3D shape ( source . origin ). 

? validate GPUExtent3D shape ( copySize ). 

Issue the subsequent steps on the Device timeline of this . [[device]] : 

Device timeline steps:

Validate the encoder state of this . If it returns false, return. 

Let aligned be true . 

Let dataLength be destination . buffer . size . 

If any of the following conditions are unsatisfied, invalidate this and return. 

validating GPUTexelCopyBufferInfo ( destination ) returns true . 

destination . buffer . usage contains
COPY_DST . 

validating texture buffer copy ( source , destination , dataLength , copySize , COPY_SRC , aligned ) returns true . 

If device. [[features]] does not contain "core-features-and-limits" :

source . texture . format must not be a compressed format . 

Enqueue a command on this which issues the subsequent steps on the
Queue timeline when executed. 

Queue timeline steps:

Let blockWidth be the texel block width of source . texture . 

Let blockHeight be the texel block height of source . texture . 

Let srcOrigin be source . origin . 

Let srcBlockOriginX be ( srcOrigin . x ÷ blockWidth ). 

Let srcBlockOriginY be ( srcOrigin . y ÷ blockHeight ). 

Let blockColumns be ( copySize . width ÷ blockWidth ). 

Let blockRows be ( copySize . height ÷ blockHeight ). 

Assert that srcBlockOriginX , srcBlockOriginY , blockColumns , and blockRows are integers. 

For each z in the range [0, copySize . depthOrArrayLayers − 1]: 

Let srcSubregion be texture copy sub-region ( z + srcOrigin . z ) of source . 

For each y in the range [0, blockRows − 1]: 

For each x in the range [0, blockColumns − 1]: 

Let blockOffset be the texel block byte offset of destination for ( x , y , z ) of
source . texture . 

Set destination . buffer at offset blockOffset to be an
equivalent texel representation to texel block 
( srcBlockOriginX + x , srcBlockOriginY + y ) of srcSubregion . 

copyTextureToTexture(source, destination, copySize) 

Encode a command into the GPUCommandEncoder that copies data from a sub-region of one
or multiple contiguous texture subresources to another sub-region of one or
multiple continuous texture subresources . 

Called on: GPUCommandEncoder this .

Arguments: 

Arguments for the GPUCommandEncoder.copyTextureToTexture(source, destination, copySize) method. 

Parameter
Type
Nullable
Optional
Description

source 
GPUTexelCopyTextureInfo 
✘ 
✘ 
Combined with copySize , defines the region of the source texture subresources .

destination 
GPUTexelCopyTextureInfo 
✘ 
✘ 
Combined with copySize , defines the region of the destination texture subresources .

copySize 
GPUExtent3D 
✘ 
✘ 

Returns: undefined 

Content timeline steps: 

? validate GPUOrigin3D shape ( source . origin ). 

? validate GPUOrigin3D shape ( destination . origin ). 

? validate GPUExtent3D shape ( copySize ). 

Issue the subsequent steps on the Device timeline of this . [[device]] : 

Device timeline steps:

Validate the encoder state of this . If it returns false, return. 

If any of the following conditions are unsatisfied, invalidate this and return. 

Let srcTexture be source . texture . 

Let dstTexture be destination . texture . 

validating GPUTexelCopyTextureInfo ( source , copySize ) returns true . 

srcTexture . usage contains COPY_SRC . 

validating GPUTexelCopyTextureInfo ( destination , copySize ) returns true . 

dstTexture . usage contains COPY_DST . 

srcTexture . sampleCount is equal to dstTexture . sampleCount . 

srcTexture . format and dstTexture . format 
must be copy-compatible . 

If srcTexture . format is a depth-stencil format: 

source . aspect and destination . aspect 
must both refer to all aspects of srcTexture . format 
and dstTexture . format , respectively. 

The set of subresources for texture copy ( source , copySize ) and
the set of subresources for texture copy ( destination , copySize ) are disjoint. 

If device. [[features]] does not contain "core-features-and-limits" :

source . texture . format must not be a compressed format . 

destination . texture . format must not be a compressed format . 

source . texture . sampleCount and destination . texture . sampleCount must be 1. 

Enqueue a command on this which issues the subsequent steps on the
Queue timeline when executed. 

Queue timeline steps:

Let blockWidth be the texel block width of source . texture . 

Let blockHeight be the texel block height of source . texture . 

Let srcOrigin be source . origin . 

Let srcBlockOriginX be ( srcOrigin . x ÷ blockWidth ). 

Let srcBlockOriginY be ( srcOrigin . y ÷ blockHeight ). 

Let dstOrigin be destination . origin . 

Let dstBlockOriginX be ( dstOrigin . x ÷ blockWidth ). 

Let dstBlockOriginY be ( dstOrigin . y ÷ blockHeight ). 

Let blockColumns be ( copySize . width ÷ blockWidth ). 

Let blockRows be ( copySize . height ÷ blockHeight ). 

Assert that srcBlockOriginX , srcBlockOriginY , dstBlockOriginX , dstBlockOriginY ,
blockColumns , and blockRows are integers. 

For each z in the range [0, copySize . depthOrArrayLayers − 1]: 

Let srcSubregion be texture copy sub-region ( z + srcOrigin . z ) of source . 

Let dstSubregion be texture copy sub-region ( z + dstOrigin . z ) of destination . 

For each y in the range [0, blockRows − 1]: 

For each x in the range [0, blockColumns − 1]: 

Set texel block ( dstBlockOriginX + x , dstBlockOriginY + y ) of
dstSubregion to be an equivalent texel representation to texel block 
( srcBlockOriginX + x , srcBlockOriginY + y ) of srcSubregion . 

13.6. Queries 

resolveQuerySet(querySet, firstQuery, queryCount, destination, destinationOffset) 

Resolves query results from a GPUQuerySet out into a range of a GPUBuffer . 

Called on: GPUCommandEncoder this.

Arguments: 

Arguments for the GPUCommandEncoder.resolveQuerySet(querySet, firstQuery, queryCount, destination, destinationOffset) method. 

Parameter
Type
Nullable
Optional
Description

querySet 
GPUQuerySet 
✘ 
✘ 

firstQuery 
GPUSize32 
✘ 
✘ 

queryCount 
GPUSize32 
✘ 
✘ 

destination 
GPUBuffer 
✘ 
✘ 

destinationOffset 
GPUSize64 
✘ 
✘ 

Returns: undefined 

Content timeline steps: 

Issue the subsequent steps on the Device timeline of
this . [[device]] . 

Device timeline steps:

Validate the encoder state of this . If it returns false, return. 

If any of the following conditions are unsatisfied, invalidate this and return. 

querySet is valid to use with this . 

destination is valid to use with this . 

destination . usage contains QUERY_RESOLVE . 

firstQuery < the number of queries in querySet . 

( firstQuery + queryCount ) ≤ the number of queries in querySet . 

destinationOffset is a multiple of 256. 

destinationOffset + 8 × queryCount ≤ destination . size . 

Enqueue a command on this which issues the subsequent steps on the
Queue timeline when executed. 

Queue timeline steps:

Let queryIndex be firstQuery . 

Let offset be destinationOffset . 

While queryIndex < firstQuery + queryCount : 

Set 8 bytes of destination , beginning at offset , to be the value of
querySet at queryIndex . 

Set queryIndex to be queryIndex + 1. 

Set offset to be offset + 8. 

13.7. Finalization 

A GPUCommandBuffer containing the commands recorded by the GPUCommandEncoder can be created
by calling finish() . Once finish() has been called the
command encoder can no longer be used. 

finish(descriptor) 

Completes recording of the commands sequence and returns a corresponding GPUCommandBuffer . 

Called on: GPUCommandEncoder this .

Arguments: 

Arguments for the GPUCommandEncoder.finish(descriptor) method. 

Parameter
Type
Nullable
Optional
Description

descriptor 
GPUCommandBufferDescriptor 
✘ 
✔ 

Returns: GPUCommandBuffer 

Content timeline steps: 

Let commandBuffer be a new GPUCommandBuffer . 

Issue the finish steps on the Device timeline of
this . [[device]] . 

Return commandBuffer . 

Device timeline finish steps :

Let validationSucceeded be true if all of the following requirements are met, and false otherwise. 

this must be valid . 

this . [[state]] must be " open ". 

this . [[debug_group_stack]] must be empty . 

Set this . [[state]] to " ended ". 

If validationSucceeded is false , then: 

Generate a validation error . 

Return an invalidated GPUCommandBuffer . 

Set commandBuffer . [[command_list]] to
this . [[commands]] . 

Set commandBuffer . [[used_bind_groups]] to
this . [[used_bind_groups]] . 

14. Programmable Passes 

interface mixin GPUBindingCommandsMixin {
undefined setBindGroup ( GPUIndex32 index , GPUBindGroup ? bindGroup ,
optional sequence < GPUBufferDynamicOffset > dynamicOffsets = []);

undefined setBindGroup ( GPUIndex32 index , GPUBindGroup ? bindGroup ,
[ AllowShared ] Uint32Array dynamicOffsetsData ,
GPUSize64 dynamicOffsetsDataStart ,
GPUSize32 dynamicOffsetsDataLength );
};

GPUBindingCommandsMixin assumes the presence of
GPUObjectBase and GPUCommandsMixin members on the same object.
It must only be included by interfaces which also include those mixins. 

GPUBindingCommandsMixin has the following device timeline properties : 

[[bind_groups]] , of type ordered map < GPUIndex32 , GPUBindGroup >, initially empty

The current GPUBindGroup for each index. 

[[dynamic_offsets]] , of type ordered map < GPUIndex32 , list < GPUBufferDynamicOffset >>, initally empty

The current dynamic offsets for each [[bind_groups]] entry. 

14.1. Bind Groups 

setBindGroup() has two overloads: 

setBindGroup(index, bindGroup, dynamicOffsets) 

Sets the current GPUBindGroup for the given index. 

Called on: GPUBindingCommandsMixin this.

Arguments: 

index , of type GPUIndex32 , non-nullable, required

The index to set the bind group at. 

bindGroup , of type GPUBindGroup , nullable, required

Bind group to use for subsequent render or compute commands. 

dynamicOffsets , of type sequence < GPUBufferDynamicOffset >, non-nullable, defaulting to [] 

Array containing buffer offsets in bytes for each entry in
bindGroup marked as buffer . hasDynamicOffset ,
ordered by GPUBindGroupLayoutEntry . binding .
See note for additional details. 

Returns: undefined 

Content timeline steps: 

Issue the subsequent steps on the Device timeline of
this . [[device]] . 

Device timeline steps:

Validate the encoder state of this . If it returns false, return. 

Let dynamicOffsetCount be 0 if bindGroup is null , or
bindGroup . [[layout]] . [[dynamicOffsetCount]] if not. 

If any of the following requirements are unmet, invalidate this and return. 

index must be <
this . [[device]] . [[limits]] . maxBindGroups . 

dynamicOffsets . size must equal dynamicOffsetCount . 

If bindGroup is null : 

Remove this . [[bind_groups]] [ index ]. 

Remove this . [[dynamic_offsets]] [ index ]. 

Otherwise: 

If any of the following requirements are unmet, invalidate this and return. 

bindGroup must be valid to use with this . 

For each dynamic binding 
( bufferBinding , bufferLayout , dynamicOffsetIndex ) in bindGroup : 

bufferBinding . offset + dynamicOffsets [ dynamicOffsetIndex ] +
bufferLayout . minBindingSize must be ≤
bufferBinding . buffer . size . 

If bufferLayout . type is "uniform" : 

dynamicOffset must be a multiple of minUniformBufferOffsetAlignment . 

If bufferLayout . type is "storage" 
or "read-only-storage" : 

dynamicOffset must be a multiple of minStorageBufferOffsetAlignment . 

Set this . [[bind_groups]] [ index ] to be bindGroup . 

Set this . [[dynamic_offsets]] [ index ] to be a copy of dynamicOffsets . 

Append bindGroup to this . [[used_bind_groups]] . 

If this is a GPURenderCommandsMixin : 

For each bindGroup in this . [[bind_groups]] ,
merge bindGroup . [[usedResources]] 
into this . [[usage scope]] 

setBindGroup(index, bindGroup, dynamicOffsetsData, dynamicOffsetsDataStart, dynamicOffsetsDataLength) 

Sets the current GPUBindGroup for the given index, specifying dynamic offsets as a subset
of a Uint32Array . 

Called on: GPUBindingCommandsMixin this .

Arguments: 

Arguments for the GPUBindingCommandsMixin.setBindGroup(index, bindGroup, dynamicOffsetsData, dynamicOffsetsDataStart, dynamicOffsetsDataLength) method. 

Parameter
Type
Nullable
Optional
Description

index 
GPUIndex32 
✘ 
✘ 
The index to set the bind group at.

bindGroup 
GPUBindGroup ? 
✔ 
✘ 
Bind group to use for subsequent render or compute commands.

dynamicOffsetsData 
Uint32Array 
✘ 
✘ 
Array containing buffer offsets in bytes for each entry in
bindGroup marked as buffer . hasDynamicOffset ,
ordered by GPUBindGroupLayoutEntry . binding .
See note for additional details.

dynamicOffsetsDataStart 
GPUSize64 
✘ 
✘ 
Offset in elements into dynamicOffsetsData where the
buffer offset data begins.

dynamicOffsetsDataLength 
GPUSize32 
✘ 
✘ 
Number of buffer offsets to read from dynamicOffsetsData .

Returns: undefined 

Content timeline steps: 

If any of the following requirements are unmet, throw a RangeError and return. 

dynamicOffsetsDataStart must be ≥ 0. 

dynamicOffsetsDataStart + dynamicOffsetsDataLength must be ≤
dynamicOffsetsData . length . 

Let dynamicOffsets be a list containing the range, starting at index
dynamicOffsetsDataStart , of dynamicOffsetsDataLength elements of
a copy of dynamicOffsetsData . 

Call this . setBindGroup ( index , bindGroup , dynamicOffsets ). 

NOTE: 

Dynamic offset are applied in GPUBindGroupLayoutEntry . binding order.

This means that if dynamic bindings is the list of each GPUBindGroupLayoutEntry in the GPUBindGroupLayout 
with buffer ?. hasDynamicOffset set to true , sorted by
GPUBindGroupLayoutEntry . binding , then dynamic offset[i] , as supplied to
setBindGroup() , will correspond to dynamic bindings[i] . 

For a GPUBindGroupLayout created with the following call:

// Note the bindings are listed out-of-order in this array, but it 
// doesn't matter because they will be sorted by binding index. 
let layout = gpuDevice . createBindGroupLayout ({ 
entries : [{ 
binding : 1 , 
buffer : {}, 
}, { 
binding : 2 , 
buffer : { dynamicOffset : true }, 
}, { 
binding : 0 , 
buffer : { dynamicOffset : true }, 
}] 
}); 

Used by a GPUBindGroup created with the following call: 

// Like above, the array order doesn't matter here. 
// It doesn't even need to match the order used in the layout. 
let bindGroup = gpuDevice . createBindGroup ({ 
layout : layout , 
entries : [{ 
binding : 1 , 
resource : { buffer : bufferA , offset : 256 }, 
}, { 
binding : 2 , 
resource : { buffer : bufferB , offset : 512 }, 
}, { 
binding : 0 , 
resource : { buffer : bufferC }, 
}] 
}); 

And bound with the following call: 

pass . setBindGroup ( 0 , bindGroup , [ 1024 , 2048 ]); 

The following buffer offsets will be applied: 

Binding 
Buffer 
Offset

0 
bufferC 
1024 (Dynamic)

1 
bufferA 
256 (Static)

2 
bufferB 
2560 (Static + Dynamic)

To Iterate over each dynamic binding offset in a given GPUBindGroup bindGroup 
with a given list of steps to be executed for each dynamic offset, run the following device timeline steps:

Let dynamicOffsetIndex be 0 . 

Let layout be bindGroup . [[layout]] . 

For each GPUBindGroupEntry entry in bindGroup . [[entries]] ordered in increasing values of entry . binding : 

Let bindingDescriptor be the GPUBindGroupLayoutEntry at
layout . [[entryMap]] [ entry . binding ]: 

If bindingDescriptor . buffer ?. hasDynamicOffset is true : 

Let bufferBinding be get as buffer binding ( entry . resource ). 

Let bufferLayout be bindingDescriptor . buffer . 

Call steps with bufferBinding , bufferLayout , and dynamicOffsetIndex . 

Let dynamicOffsetIndex be dynamicOffsetIndex + 1 

Validate encoder bind groups (encoder, pipeline)

Arguments: 

GPUBindingCommandsMixin encoder 

Encoder whose bind groups are being validated. 

GPUPipelineBase pipeline 

Pipeline to validate encoder s bind groups are compatible with. 

Device timeline steps: 

If any of the following conditions are unsatisfied, return false : 

Let bindGroupLayouts be
pipeline . [[layout]] . [[bindGroupLayouts]] . 

pipeline must not be null . 

All bind groups used by the pipeline must be set and compatible with the pipeline layout, determined as follows: 

For each pair of ( GPUIndex32 index , GPUBindGroupLayout bindGroupLayout ) in bindGroupLayouts : 

If bindGroupLayout is null , continue . 

Let bindGroup be encoder . [[bind_groups]] [ index ]. 

Let dynamicOffsets be encoder . [[dynamic_offsets]] [ index ]. 

bindGroup must not be null . 

bindGroup . [[layout]] must be group-equivalent with bindGroupLayout . 

Let dynamicOffsetIndex be 0. 

For each GPUBindGroupEntry bindGroupEntry in bindGroup . [[entries]] ,
sorted by bindGroupEntry . binding : 

Let bindGroupLayoutEntry be
bindGroup . [[layout]] . [[entryMap]] [ bindGroupEntry . binding ]. 

If bindGroupLayoutEntry . buffer is not
provided , continue . 

Let bound be get as buffer binding ( bindGroupEntry . resource ). 

If bindGroupLayoutEntry . buffer . hasDynamicOffset : 

Increment bound . offset by
dynamicOffsets [ dynamicOffsetIndex ]. 

Increment dynamicOffsetIndex by 1. 

If bindGroupEntry . [[prevalidatedSize]] is false : 

effective buffer binding size ( bound ) must be ≥ minimum buffer binding size 
of the binding variable in pipeline ’s shader that corresponds to bindGroupEntry . 

Encoder bind groups alias a writable resource ( encoder , pipeline ) must be false . 

If encoder . [[device]] . [[features]] does not contain "core-features-and-limits" :

All bindings referring to the same GPUTexture must have compatible GPUTextureView s, determined as follows: 

For each pair of ( GPUIndex32 index1 , GPUBindGroupLayout bindGroupLayout1 ) in bindGroupLayouts : 

If bindGroupLayout1 is null , continue . 

Let bindGroup1 be encoder . [[bind_groups]] [ index1 ]. 

For each GPUBindGroupEntry bindGroupEntry1 in bindGroup1 . [[entries]] : 

If bindGroupEntry1 . resource is not a GPUTextureView , continue . 

Let descriptor1 be bindGroupEntry1 . resource . [[descriptor]] . 

For each pair of ( GPUIndex32 index2 , GPUBindGroupLayout bindGroupLayout2 ) in bindGroupLayouts : 

If bindGroupLayout2 is null , continue . 

Let bindGroup2 be encoder . [[bind_groups]] [ index2 ]. 

For each GPUBindGroupEntry bindGroupEntry2 in bindGroup2 . [[entries]] : 

If bindGroupEntry2 . resource is not a GPUTextureView , continue . 

If bindGroupEntry1 . resource . [[texture]] is not equal to
bindGroupEntry2 . resource . [[texture]] , continue . 

Let descriptor2 be bindGroupEntry2 . resource . [[descriptor]] . 

descriptor2 . baseMipLevel must be equal to descriptor1 . baseMipLevel . 

descriptor2 . mipLevelCount must be equal to descriptor1 . mipLevelCount . 

descriptor2 . aspect must be equal to descriptor1 . aspect . 

descriptor2 . swizzle must be equal to descriptor1 . swizzle . 

Otherwise return true . 

Encoder bind groups alias a writable resource ( encoder , pipeline )
if any writable buffer binding range overlaps with any other binding range of the same buffer,
or any writable texture binding overlaps in texture subresources with any other texture binding
(which may use the same or a different GPUTextureView object).

Note: This algorithm limits the use of the usage scope storage exception . 

Arguments: 

GPUBindingCommandsMixin encoder 

Encoder whose bind groups are being validated. 

GPUPipelineBase pipeline 

Pipeline to validate encoder s bind groups are compatible with. 

Device timeline steps: 

For each stage in [ VERTEX , FRAGMENT , COMPUTE ]: 

Let bufferBindings be a list of ( GPUBufferBinding , boolean ) pairs,
where the latter indicates whether the resource was used as writable. 

Let textureViews be a list of ( GPUTextureView , boolean ) pairs,
where the latter indicates whether the resource was used as writable. 

For each pair of ( GPUIndex32 bindGroupIndex , GPUBindGroupLayout bindGroupLayout ) in
pipeline . [[layout]] . [[bindGroupLayouts]] : 

Let bindGroup be
encoder . [[bind_groups]] [ bindGroupIndex ]. 

Let bindGroupLayoutEntries be
bindGroupLayout . [[descriptor]] . entries . 

Let bufferRanges be the bound buffer ranges of bindGroup ,
given dynamic offsets
encoder . [[dynamic_offsets]] [ bindGroupIndex ] 

For each ( GPUBindGroupLayoutEntry bindGroupLayoutEntry ,
GPUBufferBinding resource ) in bufferRanges , in which
bindGroupLayoutEntry . visibility contains stage : 

Let resourceWritable be ( bindGroupLayoutEntry . buffer . type == "storage" ). 

For each pair ( GPUBufferBinding pastResource , boolean pastResourceWritable ) in bufferBindings : 

If ( resourceWritable or pastResourceWritable ) is true, and
pastResource and resource are buffer-binding-aliasing , return true . 

Append ( resource , resourceWritable ) to bufferBindings . 

For each GPUBindGroupLayoutEntry bindGroupLayoutEntry in
bindGroupLayoutEntries , and corresponding GPUTextureView resource 
in bindGroup , in which
bindGroupLayoutEntry . visibility contains stage : 

If bindGroupLayoutEntry . storageTexture is not provided , continue . 

Let resourceWritable be whether
bindGroupLayoutEntry . storageTexture . access 
is a writable access mode. 

For each pair ( GPUTextureView pastResource , boolean pastResourceWritable ) in textureViews , 

If ( resourceWritable or pastResourceWritable ) is true, and
pastResource and resource is texture-view-aliasing , return true . 

Append ( resource , resourceWritable ) to textureViews . 

Return false . 

Note: 
Implementations are strongly encouraged to optimize this algorithm. 

15. Debug Markers 

GPUDebugCommandsMixin provides methods to apply debug labels to groups
of commands or insert a single label into the command sequence. 

Debug groups can be nested to create a hierarchy of labeled commands, and must be well-balanced. 

Like object labels , these labels have no required behavior, but may be shown
in error messages and browser developer tools, and may be passed to native API backends. 

interface mixin GPUDebugCommandsMixin {
undefined pushDebugGroup ( USVString groupLabel );
undefined popDebugGroup ();
undefined insertDebugMarker ( USVString markerLabel );
};

GPUDebugCommandsMixin assumes the presence of
GPUObjectBase and GPUCommandsMixin members on the same object.
It must only be included by interfaces which also include those mixins. 

GPUDebugCommandsMixin has the following device timeline properties : 

[[debug_group_stack]] , of type stack < USVString >

A stack of active debug group labels. 

GPUDebugCommandsMixin has the following methods: 

pushDebugGroup(groupLabel) 

Begins a labeled debug group containing subsequent commands. 

Called on: GPUDebugCommandsMixin this .

Arguments: 

Arguments for the GPUDebugCommandsMixin.pushDebugGroup(groupLabel) method. 

Parameter
Type
Nullable
Optional
Description

groupLabel 
USVString 
✘ 
✘ 
The label for the command group.

Returns: undefined 

Content timeline steps: 

Issue the subsequent steps on the Device timeline of
this . [[device]] . 

Device timeline steps:

Validate the encoder state of this . If it returns false, return. 

Push groupLabel onto this . [[debug_group_stack]] . 

popDebugGroup() 

Ends the labeled debug group most recently started by pushDebugGroup() . 

Called on: GPUDebugCommandsMixin this .

Returns: undefined 

Content timeline steps: 

Issue the subsequent steps on the Device timeline of
this . [[device]] . 

Device timeline steps:

Validate the encoder state of this . If it returns false, return. 

If any of the following requirements are unmet, invalidate this and return. 

this . [[debug_group_stack]] must not be empty . 

Pop an entry off of this . [[debug_group_stack]] . 

insertDebugMarker(markerLabel) 

Marks a point in a stream of commands with a label. 

Called on: GPUDebugCommandsMixin this.

Arguments: 

Arguments for the GPUDebugCommandsMixin.insertDebugMarker(markerLabel) method. 

Parameter
Type
Nullable
Optional
Description

markerLabel 
USVString 
✘ 
✘ 
The label to insert.

Returns: undefined 

Content timeline steps: 

Issue the subsequent steps on the Device timeline of
this . [[device]] . 

Device timeline steps:

Validate the encoder state of this . If it returns false, return. 

16. Compute Passes 

16.1. GPUComputePassEncoder 

[ Exposed =( Window , Worker ), SecureContext ]
interface GPUComputePassEncoder {
undefined setPipeline ( GPUComputePipeline pipeline );
undefined dispatchWorkgroups ( GPUSize32 workgroupCountX , optional GPUSize32 workgroupCountY = 1, optional GPUSize32 workgroupCountZ = 1);
undefined dispatchWorkgroupsIndirect ( GPUBuffer indirectBuffer , GPUSize64 indirectOffset );

undefined end ();
};
GPUComputePassEncoder includes GPUObjectBase ;
GPUComputePassEncoder includes GPUCommandsMixin ;
GPUComputePassEncoder includes GPUDebugCommandsMixin ;
GPUComputePassEncoder includes GPUBindingCommandsMixin ;

GPUComputePassEncoder has the following device timeline properties : 

[[command_encoder]] , of type GPUCommandEncoder , readonly

The GPUCommandEncoder that created this compute pass encoder. 

[[endTimestampWrite]] , of type GPU command ?, readonly, defaulting to null 

GPU command , if any, writing a timestamp when the pass ends. 

[[pipeline]] , of type GPUComputePipeline , initially null 

The current GPUComputePipeline . 

16.1.1. Compute Pass Encoder Creation 

dictionary GPUComputePassTimestampWrites {
required GPUQuerySet querySet ;
GPUSize32 beginningOfPassWriteIndex ;
GPUSize32 endOfPassWriteIndex ;
};

querySet , of type GPUQuerySet 

The GPUQuerySet , of type "timestamp" , that the query results will be
written to. 

beginningOfPassWriteIndex , of type GPUSize32 

If defined, indicates the query index in querySet into
which the timestamp at the beginning of the compute pass will be written. 

endOfPassWriteIndex , of type GPUSize32 

If defined, indicates the query index in querySet into
which the timestamp at the end of the compute pass will be written. 

Note: Timestamp query values are written in nanoseconds, but how the value is determined is
implementation-defined . See § 20.4 Timestamp Query for details. 

dictionary GPUComputePassDescriptor 
: GPUObjectDescriptorBase {
GPUComputePassTimestampWrites timestampWrites ;
};

timestampWrites , of type GPUComputePassTimestampWrites 

Defines which timestamp values will be written for this pass, and where to write them to. 

16.1.2. Dispatch 

setPipeline(pipeline) 

Sets the current GPUComputePipeline . 

Called on: GPUComputePassEncoder this.

Arguments: 

Arguments for the GPUComputePassEncoder.setPipeline(pipeline) method. 

Parameter
Type
Nullable
Optional
Description

pipeline 
GPUComputePipeline 
✘ 
✘ 
The compute pipeline to use for subsequent dispatch commands.

Returns: undefined 

Content timeline steps: 

Issue the subsequent steps on the Device timeline of
this . [[device]] . 

Device timeline steps:

Validate the encoder state of this . If it returns false, return. 

If any of the following conditions are unsatisfied, invalidate this and return. 

pipeline is valid to use with this . 

Set this . [[pipeline]] to be pipeline . 

dispatchWorkgroups(workgroupCountX, workgroupCountY, workgroupCountZ) 

Dispatch work to be performed with the current GPUComputePipeline .
See § 23.1 Computing for the detailed specification. 

Called on: GPUComputePassEncoder this.

Arguments: 

Arguments for the GPUComputePassEncoder.dispatchWorkgroups(workgroupCountX, workgroupCountY, workgroupCountZ) method. 

Parameter
Type
Nullable
Optional
Description

workgroupCountX 
GPUSize32 
✘ 
✘ 
X dimension of the grid of workgroups to dispatch.

workgroupCountY 
GPUSize32 
✘ 
✔ 
Y dimension of the grid of workgroups to dispatch.

workgroupCountZ 
GPUSize32 
✘ 
✔ 
Z dimension of the grid of workgroups to dispatch.

NOTE: 

The x , y , and z values passed to dispatchWorkgroups() 
and dispatchWorkgroupsIndirect() are the number of
workgroups to dispatch for each dimension, not the number of shader invocations
to perform across each dimension. This matches the behavior of modern native GPU
APIs, but differs from the behavior of OpenCL.

This means that if a GPUShaderModule defines an entry point with
@workgroup_size(4, 4) , and work is dispatched to it with the call
computePass.dispatchWorkgroups(8, 8); the entry point will be invoked 1024 times
total: Dispatching a 4x4 workgroup 8 times along both the X and Y axes.
( 4*4*8*8=1024 ) 

Returns: undefined 

Content timeline steps: 

Issue the subsequent steps on the Device timeline of
this . [[device]] . 

Device timeline steps:

Validate the encoder state of this . If it returns false, return. 

Let usageScope be an empty usage scope . 

For each bindGroup in this . [[bind_groups]] ,
merge bindGroup . [[usedResources]] 
into this . [[usage scope]] 

If any of the following conditions are unsatisfied, invalidate this and return. 

usageScope must satisfy usage scope validation . 

Validate encoder bind groups ( this , this . [[pipeline]] )
is true . 

all of workgroupCountX , workgroupCountY and workgroupCountZ are ≤
this .device.limits. maxComputeWorkgroupsPerDimension . 

let workgroupSize be the computed workgroup size for
bindingState . [[pipeline]] . 

the entry point uses the workgroup_index 
built-in value and workgroupCountX × workgroupCountY 
× workgroupCountZ 
is out of range of an unsigned 32-bit integer. 

the entry point uses the global_invocation_index 
built-in value and workgroupCountX × workgroupCountY 
× workgroupCountZ × workgroupSize 
is out of range of an unsigned 32-bit integer. 

Let bindingState be a snapshot of this ’s current state. 

Enqueue a command on this which issues the subsequent steps on the
Queue timeline . 

Queue timeline steps:

Execute a grid of workgroups with dimensions [ workgroupCountX , workgroupCountY ,
workgroupCountZ ] with bindingState . [[pipeline]] using
bindingState . [[bind_groups]] . 

dispatchWorkgroupsIndirect(indirectBuffer, indirectOffset) 

Dispatch work to be performed with the current GPUComputePipeline using parameters read
from a GPUBuffer .
See § 23.1 Computing for the detailed specification. 

The indirect dispatch parameters encoded in the buffer must be a tightly
packed block of three 32-bit unsigned integer values (12 bytes total) ,
given in the same order as the arguments for dispatchWorkgroups() .
For example: 

let dispatchIndirectParameters = new Uint32Array ( 3 ); 
dispatchIndirectParameters [ 0 ] = workgroupCountX ; 
dispatchIndirectParameters [ 1 ] = workgroupCountY ; 
dispatchIndirectParameters [ 2 ] = workgroupCountZ ; 

Called on: GPUComputePassEncoder this.

Arguments: 

Arguments for the GPUComputePassEncoder.dispatchWorkgroupsIndirect(indirectBuffer, indirectOffset) method. 

Parameter
Type
Nullable
Optional
Description

indirectBuffer 
GPUBuffer 
✘ 
✘ 
Buffer containing the indirect dispatch parameters .

indirectOffset 
GPUSize64 
✘ 
✘ 
Offset in bytes into indirectBuffer where the dispatch data begins.

Returns: undefined 

Content timeline steps: 

Issue the subsequent steps on the Device timeline of
this . [[device]] . 

Device timeline steps:

Validate the encoder state of this . If it returns false, return. 

Let usageScope be an empty usage scope . 

For each bindGroup in this . [[bind_groups]] ,
merge bindGroup . [[usedResources]] 
into this . [[usage scope]] 

Add indirectBuffer to usageScope 
with usage input . 

If any of the following conditions are unsatisfied, invalidate this and return. 

usageScope must satisfy usage scope validation . 

Validate encoder bind groups ( this , this . [[pipeline]] )
is true . 

indirectBuffer is valid to use with this . 

indirectBuffer . usage contains INDIRECT . 

indirectOffset + sizeof( indirect dispatch parameters ) ≤
indirectBuffer . size . 

indirectOffset is a multiple of 4. 

Let bindingState be a snapshot of this ’s current state. 

Enqueue a command on this which issues the subsequent steps on the
Queue timeline . 

Queue timeline steps:

Let workgroupCountX be an unsigned 32-bit integer read from indirectBuffer at
indirectOffset bytes. 

Let workgroupCountY be an unsigned 32-bit integer read from indirectBuffer at
( indirectOffset + 4) bytes. 

Let workgroupCountZ be an unsigned 32-bit integer read from indirectBuffer at
( indirectOffset + 8) bytes. 

Let workgroupSize be the computed workgroup size for
bindingState . [[pipeline]] 

If workgroupCountX , workgroupCountY , or workgroupCountZ is greater than
this .device.limits. maxComputeWorkgroupsPerDimension ,
return. 

If the entry point uses the workgroup_index 
built-in value and workgroupCountX × workgroupCountY ×
workgroupCountZ is out of range of an unsigned
32-bit integer return. 

If the entry point uses the global_invocation_index 
built-in value and workgroupCountX × workgroupCountY ×
workgroupCountZ × workgroupSize is out of range of an unsigned
32-bit integer return. 

Execute a grid of workgroups with dimensions [ workgroupCountX , workgroupCountY ,
workgroupCountZ ] with bindingState . [[pipeline]] using
bindingState . [[bind_groups]] . 

16.1.3. Finalization 

The compute pass encoder can be ended by calling end() once the user
has finished recording commands for the pass. Once end() has been
called the compute pass encoder can no longer be used. 

end() 

Completes recording of the compute pass commands sequence. 

Called on: GPUComputePassEncoder this .

Returns: undefined 

Content timeline steps: 

Issue the subsequent steps on the Device timeline of
this . [[device]] . 

Device timeline steps:

Let parentEncoder be this . [[command_encoder]] . 

If any of the following requirements are unmet,
generate a validation error and return. 

this . [[state]] must be " open ". 

parentEncoder . [[state]] must be " locked ". 

Set this . [[state]] to " ended ". 

Set parentEncoder . [[state]] to " open ". 

Extend parentEncoder . [[used_bind_groups]] with
this . [[used_bind_groups]] . 

If any of the following requirements are unmet, invalidate parentEncoder and return. 

this must be valid . 

this . [[debug_group_stack]] must be empty . 

Extend parentEncoder . [[commands]] 
with this . [[commands]] . 

If this . [[endTimestampWrite]] is not null : 

Extend parentEncoder . [[commands]] 
with this . [[endTimestampWrite]] . 

17. Render Passes 

17.1. GPURenderPassEncoder 

[ Exposed =( Window , Worker ), SecureContext ]
interface GPURenderPassEncoder {
undefined setViewport ( float x , float y ,
float width , float height ,
float minDepth , float maxDepth );

undefined setScissorRect ( GPUIntegerCoordinate x , GPUIntegerCoordinate y ,
GPUIntegerCoordinate width , GPUIntegerCoordinate height );

undefined setBlendConstant ( GPUColor color );
undefined setStencilReference ( GPUStencilValue reference );

undefined beginOcclusionQuery ( GPUSize32 queryIndex );
undefined endOcclusionQuery ();

undefined executeBundles ( sequence < GPURenderBundle > bundles );
undefined end ();
};
GPURenderPassEncoder includes GPUObjectBase ;
GPURenderPassEncoder includes GPUCommandsMixin ;
GPURenderPassEncoder includes GPUDebugCommandsMixin ;
GPURenderPassEncoder includes GPUBindingCommandsMixin ;
GPURenderPassEncoder includes GPURenderCommandsMixin ;

GPURenderPassEncoder has the following device timeline properties : 

[[command_encoder]] , of type GPUCommandEncoder , readonly

The GPUCommandEncoder that created this render pass encoder. 

[[attachment_size]] , readonly

Set to the following extents: 

width, height = the dimensions of the pass’s render attachments 

[[occlusion_query_set]] , of type GPUQuerySet , readonly

The GPUQuerySet to store occlusion query results for the pass, which is initialized with
GPURenderPassDescriptor . occlusionQuerySet at pass creation time. 

[[endTimestampWrite]] , of type GPU command ?, readonly, defaulting to null 

GPU command , if any, writing a timestamp when the pass ends. 

[[maxDrawCount]] of type GPUSize64 , readonly

The maximum number of draws allowed in this pass. 

[[occlusion_query_active]] , of type boolean 

Whether the pass’s [[occlusion_query_set]] is being written. 

When executing encoded render pass commands as part of a GPUCommandBuffer , an internal
RenderState object is used to track the current state required for rendering. 

RenderState has the following queue timeline properties : 

[[occlusionQueryIndex]] , of type GPUSize32 

The index into [[occlusion_query_set]] at which to store the
occlusion query results. 

[[viewport]] 

Current viewport rectangle and depth range. Initially set to the following values: 

x, y = 0.0, 0.0 

width, height = the dimensions of the pass’s render targets 

minDepth, maxDepth = 0.0, 1.0 

[[scissorRect]] 

Current scissor rectangle. Initially set to the following values: 

x, y = 0, 0 

width, height = the dimensions of the pass’s render targets 

[[blendConstant]] , of type GPUColor 

Current blend constant value, initially [0, 0, 0, 0] . 

[[stencilReference]] , of type GPUStencilValue 

Current stencil reference value, initially 0 . 

[[colorAttachments]] , of type sequence < GPURenderPassColorAttachment ?>

The color attachments and state for this render pass. 

[[depthStencilAttachment]] , of type GPURenderPassDepthStencilAttachment ?

The depth/stencil attachment and state for this render pass. 

Render passes also have framebuffer memory , which contains the texel data associated with
each attachment that is written into by draw commands and read from for blending and depth/stencil testing. 

Note: Depending on the GPU hardware, framebuffer memory may be the memory allocated by the attachment textures or
may be a separate area of memory that the texture data is copied to and from, such as with tile-based architectures. 

17.1.1. Render Pass Encoder Creation 

dictionary GPURenderPassTimestampWrites {
required GPUQuerySet querySet ;
GPUSize32 beginningOfPassWriteIndex ;
GPUSize32 endOfPassWriteIndex ;
};

querySet , of type GPUQuerySet 

The GPUQuerySet , of type "timestamp" , that the query results will be
written to. 

beginningOfPassWriteIndex , of type GPUSize32 

If defined, indicates the query index in querySet into
which the timestamp at the beginning of the render pass will be written. 

endOfPassWriteIndex , of type GPUSize32 

If defined, indicates the query index in querySet into
which the timestamp at the end of the render pass will be written. 

Note: Timestamp query values are written in nanoseconds, but how the value is determined is
implementation-defined . See § 20.4 Timestamp Query for details. 

dictionary GPURenderPassDescriptor 
: GPUObjectDescriptorBase {
required sequence < GPURenderPassColorAttachment ?> " href="#dom-gpurenderpassdescriptor-colorattachments" id="ref-for-dom-gpurenderpassdescriptor-colorattachments⑥"> colorAttachments ;
GPURenderPassDepthStencilAttachment depthStencilAttachment ;
GPUQuerySet occlusionQuerySet ;
GPURenderPassTimestampWrites timestampWrites ;
GPUSize64 maxDrawCount = 50000000;
};

colorAttachments , of type sequence<GPURenderPassColorAttachment?> 

The set of GPURenderPassColorAttachment values in this sequence defines which
color attachments will be output to when executing this render pass. 

Due to usage compatibility , no color attachment
may alias another attachment or any resource used inside the render pass. 

depthStencilAttachment , of type GPURenderPassDepthStencilAttachment 

The GPURenderPassDepthStencilAttachment value that defines the depth/stencil
attachment that will be output to and tested against when executing this render pass. 

Due to usage compatibility , no writable depth/stencil attachment
may alias another attachment or any resource used inside the render pass. 

occlusionQuerySet , of type GPUQuerySet 

The GPUQuerySet value defines where the occlusion query results will be stored for this pass. 

timestampWrites , of type GPURenderPassTimestampWrites 

Defines which timestamp values will be written for this pass, and where to write them to. 

maxDrawCount , of type GPUSize64 , defaulting to 50000000 

The maximum number of draw calls that will be done in the render pass. Used by some
implementations to size work injected before the render pass. Keeping the default value
is a good default, unless it is known that more draw calls will be done. 

Valid Usage 

Given a GPUDevice device and GPURenderPassDescriptor this , the following validation rules apply: 

this . colorAttachments . size must be ≤
device . [[limits]] . maxColorAttachments . 

For each non- null colorAttachment in this . colorAttachments : 

colorAttachment . view must be valid to use with device . 

If colorAttachment . resolveTarget is provided : 

colorAttachment . resolveTarget must be valid to use with device . 

colorAttachment must meet the GPURenderPassColorAttachment Valid Usage rules. 

If this . depthStencilAttachment is provided : 

this . depthStencilAttachment . view must be valid to use with device . 

this . depthStencilAttachment must meet the GPURenderPassDepthStencilAttachment Valid Usage rules. 

There must exist at least one attachment, either: 

A non- null value in this . colorAttachments , or 

A this . depthStencilAttachment . 

Validating GPURenderPassDescriptor’s color attachment bytes per sample ( device , this . colorAttachments ) succeeds. 

All view s in non- null members of this . colorAttachments ,
and this . depthStencilAttachment . view 
if present, must have equal sampleCount s. 

For each view in non- null members of this . colorAttachments 
and this . depthStencilAttachment . view ,
if present, the [[renderExtent]] must match. 

If this . occlusionQuerySet is provided : 

this . occlusionQuerySet must be valid to use with device . 

this . occlusionQuerySet . type 
must be occlusion . 

If this . timestampWrites is provided : 

Validate timestampWrites ( device , this . timestampWrites )
must return true. 

Validating GPURenderPassDescriptor’s color attachment bytes per sample ( device , colorAttachments )

Arguments: 

GPUDevice device 

sequence < GPURenderPassColorAttachment ?> colorAttachments 

Device timeline steps: 

Let formats be an empty list < GPUTextureFormat ?> 

For each colorAttachment in colorAttachments : 

If colorAttachment is undefined , continue. 

Append colorAttachment . view . [[descriptor]] . format to formats . 

Calculating color attachment bytes per sample ( formats ) must be ≤ device . [[limits]] . maxColorAttachmentBytesPerSample . 

17.1.1.1. Color Attachments 

dictionary GPURenderPassColorAttachment {
required ( GPUTexture or GPUTextureView ) view ;
GPUIntegerCoordinate depthSlice ;
( GPUTexture or GPUTextureView ) resolveTarget ;

GPUColor clearValue ;
required GPULoadOp loadOp ;
required GPUStoreOp storeOp ;
};

view , of type (GPUTexture or GPUTextureView) 

Describes the texture subresource that will be output to for this color attachment.
The subresource is determined by calling get as texture view ( view ). 

depthSlice , of type GPUIntegerCoordinate 

Indicates the depth slice index of "3d" view 
that will be output to for this color attachment. 

resolveTarget , of type (GPUTexture or GPUTextureView) 

Describes the texture subresource that will receive the resolved output for this color
attachment if view is multisampled.
The subresource is determined by calling get as texture view ( resolveTarget ). 

clearValue , of type GPUColor 

Indicates the value to clear view to prior to executing the
render pass. If not provided , defaults to {r: 0, g: 0, b: 0, a: 0} . Ignored
if loadOp is not "clear" . 

The components of clearValue are all double values.
They are converted to a texel value of texture format matching the render attachment.
If conversion fails, a validation error is generated. 

loadOp , of type GPULoadOp 

Indicates the load operation to perform on view prior to
executing the render pass. 

Note: It is recommended to prefer clearing; see "clear" for details. 

storeOp , of type GPUStoreOp 

The store operation to perform on view 
after executing the render pass. 

GPURenderPassColorAttachment Valid Usage 

Given a GPURenderPassColorAttachment this : 

Let renderViewDescriptor be this . view . [[descriptor]] . 

Let renderTexture be this . view . [[texture]] . 

All of the requirements in the following steps must be met. 

renderViewDescriptor . format must be a color renderable format . 

this . view must be a renderable texture view . 

If renderViewDescriptor . dimension is "3d" : 

this . depthSlice must be provided and must 
be < the depthOrArrayLayers of the logical miplevel-specific texture extent 
of the renderTexture subresource at mipmap level renderViewDescriptor . baseMipLevel . 

Otherwise: 

this . depthSlice must not be provided . 

If renderViewDescriptor . usage includes the TRANSIENT_ATTACHMENT bit: 

this . loadOp must be "clear" . 

this . storeOp must be "discard" . 

If this . loadOp is "clear" : 

Converting the IDL value this . clearValue 
to a texel value of texture format renderViewDescriptor . format 
must not throw a TypeError . 

Note: An error is not thrown if the value is out-of-range for the format but in-range for
the corresponding WGSL primitive type ( f32 , i32 , or u32 ). 

If this . resolveTarget is provided : 

Let resolveViewDescriptor be this . resolveTarget . [[descriptor]] . 

Let resolveTexture be this . resolveTarget . [[texture]] . 

renderTexture . sampleCount must be > 1. 

resolveTexture . sampleCount must be 1. 

this . resolveTarget must be a non-3d renderable texture view . 

this . resolveTarget . [[renderExtent]] and
this . view . [[renderExtent]] must match. 

resolveViewDescriptor . format must equal
renderViewDescriptor . format . 

resolveTexture . format must equal
renderTexture . format . 

resolveViewDescriptor . format must support resolve according to § 26.1.1 Plain color formats . 

A GPUTextureView view is a renderable texture view 
if the all of the requirements in the following device timeline steps are met:

Let descriptor be view . [[descriptor]] . 

descriptor . usage 
must contain RENDER_ATTACHMENT . 

descriptor . dimension must be "2d" 
or "2d-array" or "3d" . 

descriptor . mipLevelCount must be 1. 

descriptor . arrayLayerCount must be 1. 

descriptor . aspect must refer to all aspects of
view . [[texture]] . 

descriptor . swizzle must be "rgba" . 

Calculating color attachment bytes per sample ( formats )

Arguments: 

sequence < GPUTextureFormat ?> formats 

Returns: GPUSize32 

Let total be 0. 

For each non-null format in formats 

Assert : format is a color renderable format . 

Let renderTargetPixelByteCost be the render target pixel byte cost of format . 

Let renderTargetComponentAlignment be the render target component alignment of format . 

Round total up to the smallest multiple of renderTargetComponentAlignment greater than or equal to total . 

Add renderTargetPixelByteCost to total . 

Return total . 

17.1.1.2. Depth/Stencil Attachments 

dictionary GPURenderPassDepthStencilAttachment {
required ( GPUTexture or GPUTextureView ) view ;

float depthClearValue ;
GPULoadOp depthLoadOp ;
GPUStoreOp depthStoreOp ;
boolean depthReadOnly = false ;

GPUStencilValue stencilClearValue = 0;
GPULoadOp stencilLoadOp ;
GPUStoreOp stencilStoreOp ;
boolean stencilReadOnly = false ;
};

view , of type (GPUTexture or GPUTextureView) 

Describes the texture subresource that will be output to and read from for this
depth/stencil attachment.
The subresource is determined by calling get as texture view ( view ). 

depthClearValue , of type float 

Indicates the value to clear view ’s depth component
to prior to executing the render pass. Ignored if depthLoadOp 
is not "clear" . Must be between 0.0 and 1.0, inclusive. 

depthLoadOp , of type GPULoadOp 

Indicates the load operation to perform on view ’s
depth component prior to executing the render pass. 

Note: It is recommended to prefer clearing; see "clear" for details. 

depthStoreOp , of type GPUStoreOp 

The store operation to perform on view ’s
depth component after executing the render pass. 

depthReadOnly , of type boolean , defaulting to false 

Indicates that the depth component of view 
is read only. 

stencilClearValue , of type GPUStencilValue , defaulting to 0 

Indicates the value to clear view ’s stencil component
to prior to executing the render pass. Ignored if stencilLoadOp 
is not "clear" . 

The value will be converted to the type of the stencil aspect of view by taking the same
number of LSBs as the number of bits in the stencil aspect of one texel of view . 

stencilLoadOp , of type GPULoadOp 

Indicates the load operation to perform on view ’s
stencil component prior to executing the render pass. 

Note: It is recommended to prefer clearing; see "clear" for details. 

stencilStoreOp , of type GPUStoreOp 

The store operation to perform on view ’s
stencil component after executing the render pass. 

stencilReadOnly , of type boolean , defaulting to false 

Indicates that the stencil component of view 
is read only. 

GPURenderPassDepthStencilAttachment Valid Usage 

Given a GPURenderPassDepthStencilAttachment this : 

Let format be this . view . [[descriptor]] . format . 

Let usage be this . view . [[descriptor]] . usage . 

All of the requirements in the following steps must be met. 

this . view must have a depth-or-stencil format . 

this . view must be a renderable texture view . 

If this . depthLoadOp is "clear" ,
this . depthClearValue must be provided and must be between 0.0 and 1.0,
inclusive. 

If format has a depth aspect and this . depthReadOnly is false : 

this . depthLoadOp must be provided . 

this . depthStoreOp must be provided . 

Otherwise: 

this . depthLoadOp must not be provided . 

this . depthStoreOp must not be provided . 

If format has a stencil aspect and this . stencilReadOnly is false : 

this . stencilLoadOp must be provided . 

this . stencilStoreOp must be provided . 

Otherwise: 

this . stencilLoadOp must not be provided . 

this . stencilStoreOp must not be provided . 

If usage includes the TRANSIENT_ATTACHMENT bit: 

If format has a depth aspect: 

this . depthLoadOp must be "clear" . 

this . depthStoreOp must be "discard" . 

If format has a stencil aspect: 

this . stencilLoadOp must be "clear" . 

this . stencilStoreOp must be "discard" . 

17.1.1.3. Load & Store Operations 

enum GPULoadOp {
"load" ,
"clear" ,
};

"load" 

Loads the existing value for this attachment into the render pass. 

"clear" 

Loads a clear value for this attachment into the render pass. 

Note: 
On some GPU hardware (primarily mobile), "clear" is significantly cheaper
because it avoids loading data from main memory into tile-local memory.
On other GPU hardware, there isn’t a significant difference. As a result, it is
recommended to use "clear" rather than "load" in cases where the
initial value doesn’t matter (e.g. the render target will be cleared using a skybox). 

enum GPUStoreOp {
"store" ,
"discard" ,
};

"store" 

Stores the resulting value of the render pass for this attachment. 

"discard" 

Discards the resulting value of the render pass for this attachment. 

Note: Discarded attachments
behave as if they are cleared to zero, but implementations are not required to perform a
clear at the end of the render pass. Implementations which do not explicitly clear discarded
attachments at the end of a pass must lazily clear them prior to the reading the attachment
contents, which occurs via sampling, copies, attaching to a later render pass with
"load" , displaying or reading back the canvas
( get a copy of the image contents of a context ), etc. 

17.1.1.4. Render Pass Layout 

GPURenderPassLayout declares the layout of the render targets of a GPURenderBundle .
It is also used internally to describe
GPURenderPassEncoder layouts and
GPURenderPipeline layouts .
It determines compatibility between render passes, render bundles, and render pipelines. 

dictionary GPURenderPassLayout 
: GPUObjectDescriptorBase {
required sequence < GPUTextureFormat ?> " href="#dom-gpurenderpasslayout-colorformats" id="ref-for-dom-gpurenderpasslayout-colorformats①"> colorFormats ;
GPUTextureFormat depthStencilFormat ;
GPUSize32 sampleCount = 1;
};

colorFormats , of type sequence<GPUTextureFormat?> 

A list of the GPUTextureFormat s of the color attachments for this pass or bundle. 

depthStencilFormat , of type GPUTextureFormat 

The GPUTextureFormat of the depth/stencil attachment for this pass or bundle. 

sampleCount , of type GPUSize32 , defaulting to 1 

Number of samples per pixel in the attachments for this pass or bundle. 

Two GPURenderPassLayout values are equal if:

Their depthStencilFormat and sampleCount are equal, and 

Their colorFormats are equal ignoring any trailing null s. 

derive render targets layout from pass 

Arguments: 

GPURenderPassDescriptor descriptor 

Returns: GPURenderPassLayout 

Device timeline steps: 

Let layout be a new GPURenderPassLayout object. 

For each colorAttachment in descriptor . colorAttachments : 

If colorAttachment is not null : 

Set layout . sampleCount to colorAttachment . view . [[texture]] . sampleCount . 

Append colorAttachment . view . [[descriptor]] . format to layout . colorFormats . 

Otherwise: 

Append null to layout . colorFormats . 

Let depthStencilAttachment be descriptor . depthStencilAttachment . 

If depthStencilAttachment is not null : 

Let view be depthStencilAttachment . view . 

Set layout . sampleCount to view . [[texture]] . sampleCount . 

Set layout . depthStencilFormat to view . [[descriptor]] . format . 

Return layout . 

derive render targets layout from pipeline 

Arguments: 

GPURenderPipelineDescriptor descriptor 

Returns: GPURenderPassLayout 

Device timeline steps: 

Let layout be a new GPURenderPassLayout object. 

Set layout . sampleCount to descriptor . multisample . count . 

If descriptor . depthStencil is provided : 

Set layout . depthStencilFormat to descriptor . depthStencil . format . 

If descriptor . fragment is provided : 

For each colorTarget in descriptor . fragment . targets : 

Append colorTarget . format to layout . colorFormats 
if colorTarget is not null , or append null otherwise. 

Return layout . 

17.1.2. Finalization 

The render pass encoder can be ended by calling end() once the user
has finished recording commands for the pass. Once end() has been
called the render pass encoder can no longer be used. 

end() 

Completes recording of the render pass commands sequence. 

Called on: GPURenderPassEncoder this .

Returns: undefined 

Content timeline steps: 

Issue the subsequent steps on the Device timeline of
this . [[device]] . 

Device timeline steps:

Let parentEncoder be this . [[command_encoder]] . 

If any of the following requirements are unmet,
generate a validation error and return. 

this . [[state]] must be " open ". 

parentEncoder . [[state]] must be " locked ". 

Set this . [[state]] to " ended ". 

Set parentEncoder . [[state]] to " open ". 

Extend parentEncoder . [[used_bind_groups]] with
this . [[used_bind_groups]] . 

If any of the following requirements are unmet, invalidate parentEncoder and return. 

this must be valid . 

this . [[usage scope]] must satisfy usage scope validation . 

this . [[debug_group_stack]] must be empty . 

this . [[occlusion_query_active]] must be false . 

this . [[drawCount]] must be ≤ this . [[maxDrawCount]] . 

Extend parentEncoder . [[commands]] 
with this . [[commands]] . 

If this . [[endTimestampWrite]] is not null : 

Extend parentEncoder . [[commands]] 
with this . [[endTimestampWrite]] . 

Enqueue a render command on this which issues the subsequent steps on the
Queue timeline with renderState when executed. 

Queue timeline steps:

For each non- null colorAttachment in renderState . [[colorAttachments]] : 

Let colorView be colorAttachment . view . 

If colorView . [[descriptor]] . dimension is: 

"3d" 

Let colorSubregion be colorAttachment . depthSlice of
colorView . 

Otherwise

Let colorSubregion be colorView . 

If colorAttachment . resolveTarget is not null : 

Resolve the multiple samples of every texel of colorSubregion to a single
sample and copy to colorAttachment . resolveTarget . 

If colorAttachment . storeOp is: 

"store" 

Ensure the contents of the framebuffer memory associated with colorSubregion 
are stored in colorSubregion . 

"discard" 

Set every texel of colorSubregion to zero. 

Let depthStencilAttachment be renderState . [[depthStencilAttachment]] . 

If depthStencilAttachment is not null : 

If depthStencilAttachment . depthStoreOp is: 

Not provided 

Assert that depthStencilAttachment . depthReadOnly 
is true and leave the depth subresource of depthStencilView 
unchanged. 

"store" 

Ensure the contents of the framebuffer memory associated with the depth 
subresource of depthStencilView are stored in depthStencilView . 

"discard" 

Set every texel in the depth subresource 
of depthStencilView to zero. 

If depthStencilAttachment . stencilStoreOp is: 

Not provided 

Assert that depthStencilAttachment . stencilReadOnly 
is true and leave the stencil subresource of depthStencilView 
unchanged. 

"store" 

Ensure the contents of the framebuffer memory associated with the stencil 
subresource of depthStencilView are stored in depthStencilView . 

"discard" 

Set every texel in the stencil subresource 
depthStencilView to zero. 

Let renderState be null . 

Note: Discarded attachments behave as if they are cleared to zero, but implementations are not required
to perform a clear at the end of the render pass. See the note on "discard" for
additional details. 

Note: Read-only depth-stencil attachments can be thought of as implicitly using the "store" 
operation, but since their content is unchanged during the render pass implementations don’t need to
update the attachment. Validation that requires the store op to not be provided for read-only attachments
is done in GPURenderPassDepthStencilAttachment Valid Usage . 

17.2. GPURenderCommandsMixin 

GPURenderCommandsMixin defines rendering commands common to GPURenderPassEncoder 
and GPURenderBundleEncoder . 

interface mixin GPURenderCommandsMixin {
undefined setPipeline ( GPURenderPipeline pipeline );

undefined setIndexBuffer ( GPUBuffer buffer , GPUIndexFormat indexFormat , optional GPUSize64 offset = 0, optional GPUSize64 size );
undefined setVertexBuffer ( GPUIndex32 slot , GPUBuffer ? buffer , optional GPUSize64 offset = 0, optional GPUSize64 size );

undefined draw ( GPUSize32 vertexCount , optional GPUSize32 instanceCount = 1,
optional GPUSize32 firstVertex = 0, optional GPUSize32 firstInstance = 0);
undefined drawIndexed ( GPUSize32 indexCount , optional GPUSize32 instanceCount = 1,
optional GPUSize32 firstIndex = 0,
optional GPUSignedOffset32 baseVertex = 0,
optional GPUSize32 firstInstance = 0);

undefined drawIndirect ( GPUBuffer indirectBuffer , GPUSize64 indirectOffset );
undefined drawIndexedIndirect ( GPUBuffer indirectBuffer , GPUSize64 indirectOffset );
};

GPURenderCommandsMixin assumes the presence of
GPUObjectBase , GPUCommandsMixin , and GPUBindingCommandsMixin members on the same object.
It must only be included by interfaces which also include those mixins. 

GPURenderCommandsMixin has the following device timeline properties : 

[[layout]] , of type GPURenderPassLayout , readonly

The layout of the render pass. 

[[depthReadOnly]] , of type boolean , readonly

If true , indicates that the depth component is not modified. 

[[stencilReadOnly]] , of type boolean , readonly

If true , indicates that the stencil component is not modified. 

[[usage scope]] , of type usage scope , initially empty

The usage scope for this render pass or bundle. 

[[pipeline]] , of type GPURenderPipeline , initially null 

The current GPURenderPipeline . 

[[index_buffer]] , of type GPUBuffer , initially null 

The current buffer to read index data from. 

[[index_format]] , of type GPUIndexFormat 

The format of the index data in [[index_buffer]] . 

[[index_buffer_offset]] , of type GPUSize64 

The offset in bytes of the section of [[index_buffer]] currently set. 

[[index_buffer_size]] , of type GPUSize64 

The size in bytes of the section of [[index_buffer]] currently set,
initially 0 . 

[[vertex_buffers]] , of type ordered map <slot, GPUBuffer >, initially empty

The current GPUBuffer s to read vertex data from for each slot. 

[[vertex_buffer_sizes]] , of type ordered map <slot, GPUSize64 >, initially empty

The size in bytes of the section of GPUBuffer currently set for each slot. 

[[drawCount]] , of type GPUSize64 

The number of draw commands recorded in this encoder. 

To Enqueue a render command on GPURenderCommandsMixin encoder which
issues the steps of a GPU Command command with RenderState renderState , run the
following device timeline steps:

Append command to encoder . [[commands]] . 

When command is executed as part of a GPUCommandBuffer commandBuffer : 

Issue the steps of command with commandBuffer . [[renderState]] as renderState . 

17.2.1. Drawing 

setPipeline(pipeline) 

Sets the current GPURenderPipeline . 

Called on: GPURenderCommandsMixin this.

Arguments: 

Arguments for the GPURenderCommandsMixin.setPipeline(pipeline) method. 

Parameter
Type
Nullable
Optional
Description

pipeline 
GPURenderPipeline 
✘ 
✘ 
The render pipeline to use for subsequent drawing commands.

Returns: undefined 

Content timeline steps: 

Issue the subsequent steps on the Device timeline of
this . [[device]] . 

Device timeline steps:

Validate the encoder state of this . If it returns false, return. 

Let pipelineTargetsLayout be derive render targets layout from pipeline ( pipeline . [[descriptor]] ). 

If any of the following conditions are unsatisfied, invalidate this and return. 

pipeline is valid to use with this . 

this . [[layout]] equals pipelineTargetsLayout . 

If pipeline . [[writesDepth]] :
this . [[depthReadOnly]] must be false . 

If pipeline . [[writesStencil]] :
this . [[stencilReadOnly]] must be false . 

Set this . [[pipeline]] to be pipeline . 

setIndexBuffer(buffer, indexFormat, offset, size) 

Sets the current index buffer. 

Called on: GPURenderCommandsMixin this.

Arguments: 

Arguments for the GPURenderCommandsMixin.setIndexBuffer(buffer, indexFormat, offset, size) method. 

Parameter
Type
Nullable
Optional
Description

buffer 
GPUBuffer 
✘ 
✘ 
Buffer containing index data to use for subsequent drawing commands.

indexFormat 
GPUIndexFormat 
✘ 
✘ 
Format of the index data contained in buffer .

offset 
GPUSize64 
✘ 
✔ 
Offset in bytes into buffer where the index data begins. Defaults to 0 .

size 
GPUSize64 
✘ 
✔ 
Size in bytes of the index data in buffer .
Defaults to the size of the buffer minus the offset.

Returns: undefined 

Content timeline steps: 

Issue the subsequent steps on the Device timeline of this . [[device]] . 

Device timeline steps:

Validate the encoder state of this . If it returns false, return. 

If size is missing, set size to max(0, buffer . size - offset ). 

If any of the following conditions are unsatisfied, invalidate this and return. 

buffer is valid to use with this . 

buffer . usage contains INDEX . 

offset is a multiple of indexFormat ’s byte size. 

offset + size ≤ buffer . size . 

Add buffer to [[usage scope]] 
with usage input . 

Set this . [[index_buffer]] to be buffer . 

Set this . [[index_format]] to be indexFormat . 

Set this . [[index_buffer_offset]] to be offset . 

Set this . [[index_buffer_size]] to be size . 

setVertexBuffer(slot, buffer, offset, size) 

Sets the current vertex buffer for the given slot. 

Called on: GPURenderCommandsMixin this.

Arguments: 

Arguments for the GPURenderCommandsMixin.setVertexBuffer(slot, buffer, offset, size) method. 

Parameter
Type
Nullable
Optional
Description

slot 
GPUIndex32 
✘ 
✘ 
The vertex buffer slot to set the vertex buffer for.

buffer 
GPUBuffer ? 
✔ 
✘ 
Buffer containing vertex data to use for subsequent drawing commands.

offset 
GPUSize64 
✘ 
✔ 
Offset in bytes into buffer where the vertex data begins. Defaults to 0 .

size 
GPUSize64 
✘ 
✔ 
Size in bytes of the vertex data in buffer .
Defaults to the size of the buffer minus the offset.

Returns: undefined 

Content timeline steps: 

Issue the subsequent steps on the Device timeline of this . [[device]] . 

Device timeline steps:

Validate the encoder state of this . If it returns false, return. 

Let bufferSize be 0 if buffer is null , or buffer . size if not. 

If size is missing, set size to max(0, bufferSize - offset ). 

If any of the following requirements are unmet, invalidate this and return. 

slot must be <
this . [[device]] . [[limits]] . maxVertexBuffers . 

offset must be a multiple of 4. 

offset + size must be ≤ bufferSize . 

If buffer is null : 

Remove this . [[vertex_buffers]] [ slot ]. 

Remove this . [[vertex_buffer_sizes]] [ slot ]. 

Otherwise: 

If any of the following requirements are unmet, invalidate this and return. 

buffer must be valid to use with this . 

buffer . usage must contain VERTEX . 

Add buffer to [[usage scope]] 
with usage input . 

Set this . [[vertex_buffers]] [ slot ] to be buffer . 

Set this . [[vertex_buffer_sizes]] [ slot ] to be size . 

draw(vertexCount, instanceCount, firstVertex, firstInstance) 

Draws primitives.
See § 23.2 Rendering for the detailed specification. 

Called on: GPURenderCommandsMixin this.

Arguments: 

Arguments for the GPURenderCommandsMixin.draw(vertexCount, instanceCount, firstVertex, firstInstance) method. 

Parameter
Type
Nullable
Optional
Description

vertexCount 
GPUSize32 
✘ 
✘ 
The number of vertices to draw.

instanceCount 
GPUSize32 
✘ 
✔ 
The number of instances to draw.

firstVertex 
GPUSize32 
✘ 
✔ 
Offset into the vertex buffers, in vertices, to begin drawing from.

firstInstance 
GPUSize32 
✘ 
✔ 
First instance to draw.

Returns: undefined 

Content timeline steps: 

Issue the subsequent steps on the Device timeline of this . [[device]] . 

Device timeline steps:

Validate the encoder state of this . If it returns false, return. 

All of the requirements in the following steps must be met.
If any are unmet, invalidate this and return. 

It must be valid to draw with this . 

Let buffers be this . [[pipeline]] . [[descriptor]] . vertex . buffers . 

For each GPUIndex32 slot from 0 to buffers . size (non-inclusive): 

If buffers [ slot ] is null , continue . 

Let bufferSize be this . [[vertex_buffer_sizes]] [ slot ]. 

Let stride be buffers [ slot ]. arrayStride . 

Let attributes be buffers [ slot ]. attributes 

Let lastStride be the maximum value of
( attribute . offset + byteSize ( attribute . format ))
over each attribute in attributes , or 0 if attributes is empty . 

Let strideCount be computed based on buffers [ slot ]. stepMode : 

"vertex" 

firstVertex + vertexCount 

"instance" 

firstInstance + instanceCount 

If strideCount ≠ 0 : 

( strideCount − 1 ) × stride + lastStride must be ≤ bufferSize . 

Increment this . [[drawCount]] by 1. 

Let bindingState be a snapshot of this ’s current state. 

Enqueue a render command on this which issues the subsequent steps on the
Queue timeline with renderState when executed. 

Queue timeline steps:

Draw instanceCount instances, starting with instance firstInstance , of
primitives consisting of vertexCount vertices, starting with vertex firstVertex ,
with the states from bindingState and renderState . 

drawIndexed(indexCount, instanceCount, firstIndex, baseVertex, firstInstance) 

Draws indexed primitives.
See § 23.2 Rendering for the detailed specification. 

Called on: GPURenderCommandsMixin this.

Arguments: 

Arguments for the GPURenderCommandsMixin.drawIndexed(indexCount, instanceCount, firstIndex, baseVertex, firstInstance) method. 

Parameter
Type
Nullable
Optional
Description

indexCount 
GPUSize32 
✘ 
✘ 
The number of indices to draw.

instanceCount 
GPUSize32 
✘ 
✔ 
The number of instances to draw.

firstIndex 
GPUSize32 
✘ 
✔ 
Offset into the index buffer, in indices, begin drawing from.

baseVertex 
GPUSignedOffset32 
✘ 
✔ 
Added to each index value before indexing into the vertex buffers.

firstInstance 
GPUSize32 
✘ 
✔ 
First instance to draw.

Returns: undefined 

Content timeline steps: 

Issue the subsequent steps on the Device timeline of this . [[device]] . 

Device timeline steps:

Validate the encoder state of this . If it returns false, return. 

If any of the following conditions are unsatisfied, invalidate this and return. 

It is valid to draw indexed with this . 

firstIndex + indexCount ≤ this . [[index_buffer_size]] 
÷ this . [[index_format]] ’s byte size; 

Let buffers be this . [[pipeline]] . [[descriptor]] . vertex . buffers . 

For each GPUIndex32 slot from 0 to buffers . size (non-inclusive): 

If buffers [ slot ] is null , continue . 

Let bufferSize be this . [[vertex_buffer_sizes]] [ slot ]. 

Let stride be buffers [ slot ]. arrayStride . 

Let lastStride be max( attribute . offset + byteSize ( attribute . format ))
for each attribute in buffers [ slot ]. attributes . 

Let strideCount be firstInstance + instanceCount . 

If buffers [ slot ]. stepMode is "instance" and strideCount ≠ 0 : 

Ensure ( strideCount − 1 ) × stride + lastStride ≤ bufferSize . 

Increment this . [[drawCount]] by 1. 

Let bindingState be a snapshot of this ’s current state. 

Enqueue a render command on this which issues the subsequent steps on the
Queue timeline with renderState when executed. 

Queue timeline steps:

Draw instanceCount instances, starting with instance firstInstance , of
primitives consisting of indexCount indexed vertices, starting with index
firstIndex from vertex baseVertex ,
with the states from bindingState and renderState . 

Note: 
WebGPU applications should never use index data with indices out of bounds of any
bound vertex buffer that has GPUVertexStepMode "vertex" .
WebGPU implementations have different ways of handling this,
and therefore a range of behaviors is allowed.
Either the whole draw call is discarded, or the access to those attributes
out of bounds is described by WGSL’s invalid memory reference . 

drawIndirect(indirectBuffer, indirectOffset) 

Draws primitives using parameters read from a GPUBuffer .
See § 23.2 Rendering for the detailed specification. 

The indirect draw parameters encoded in the buffer must be a tightly
packed block of four 32-bit unsigned integer values (16 bytes total) , given in the same
order as the arguments for draw() . For example: 

let drawIndirectParameters = new Uint32Array ( 4 ); 
drawIndirectParameters [ 0 ] = vertexCount ; 
drawIndirectParameters [ 1 ] = instanceCount ; 
drawIndirectParameters [ 2 ] = firstVertex ; 
drawIndirectParameters [ 3 ] = firstInstance ; 

The value corresponding to firstInstance must be 0, unless the "indirect-first-instance" 
feature is enabled. If the "indirect-first-instance" feature is not enabled and
firstInstance is not zero the drawIndirect() call will be treated as a no-op. 

Called on: GPURenderCommandsMixin this.

Arguments: 

Arguments for the GPURenderCommandsMixin.drawIndirect(indirectBuffer, indirectOffset) method. 

Parameter
Type
Nullable
Optional
Description

indirectBuffer 
GPUBuffer 
✘ 
✘ 
Buffer containing the indirect draw parameters .

indirectOffset 
GPUSize64 
✘ 
✘ 
Offset in bytes into indirectBuffer where the drawing data begins.

Returns: undefined 

Content timeline steps: 

Issue the subsequent steps on the Device timeline of this . [[device]] . 

Device timeline steps:

Validate the encoder state of this . If it returns false, return. 

If any of the following conditions are unsatisfied, invalidate this and return. 

It is valid to draw with this . 

indirectBuffer is valid to use with this . 

indirectBuffer . usage contains INDIRECT . 

indirectOffset + sizeof( indirect draw parameters ) ≤
indirectBuffer . size . 

indirectOffset is a multiple of 4. 

Add indirectBuffer to [[usage scope]] 
with usage input . 

Increment this . [[drawCount]] by 1. 

Let bindingState be a snapshot of this ’s current state. 

Enqueue a render command on this which issues the subsequent steps on the
Queue timeline with renderState when executed. 

Queue timeline steps:

Let vertexCount be an unsigned 32-bit integer read from indirectBuffer at
indirectOffset bytes. 

Let instanceCount be an unsigned 32-bit integer read from indirectBuffer at
( indirectOffset + 4) bytes. 

Let firstVertex be an unsigned 32-bit integer read from indirectBuffer at
( indirectOffset + 8) bytes. 

Let firstInstance be an unsigned 32-bit integer read from indirectBuffer at
( indirectOffset + 12) bytes. 

Draw instanceCount instances, starting with instance firstInstance , of
primitives consisting of vertexCount vertices, starting with vertex firstVertex ,
with the states from bindingState and renderState . 

drawIndexedIndirect(indirectBuffer, indirectOffset) 

Draws indexed primitives using parameters read from a GPUBuffer .
See § 23.2 Rendering for the detailed specification. 

The indirect drawIndexed parameters encoded in the buffer must be a
tightly packed block of five 32-bit values (20 bytes total) , given in the same order as
the arguments for drawIndexed() . The value corresponding to
baseVertex is a signed 32-bit integer, and all others are unsigned 32-bit integers.
For example: 

let drawIndexedIndirectParameters = new Uint32Array ( 5 ); 
let drawIndexedIndirectParametersSigned = new Int32Array ( drawIndexedIndirectParameters . buffer ); 
drawIndexedIndirectParameters [ 0 ] = indexCount ; 
drawIndexedIndirectParameters [ 1 ] = instanceCount ; 
drawIndexedIndirectParameters [ 2 ] = firstIndex ; 
// baseVertex is a signed value. 
drawIndexedIndirectParametersSigned [ 3 ] = baseVertex ; 
drawIndexedIndirectParameters [ 4 ] = firstInstance ; 

The value corresponding to firstInstance must be 0, unless the "indirect-first-instance" 
feature is enabled. If the "indirect-first-instance" feature is not enabled and
firstInstance is not zero the drawIndexedIndirect() call will be treated as a no-op. 

Called on: GPURenderCommandsMixin this.

Arguments: 

Arguments for the GPURenderCommandsMixin.drawIndexedIndirect(indirectBuffer, indirectOffset) method. 

Parameter
Type
Nullable
Optional
Description

indirectBuffer 
GPUBuffer 
✘ 
✘ 
Buffer containing the indirect drawIndexed parameters .

indirectOffset 
GPUSize64 
✘ 
✘ 
Offset in bytes into indirectBuffer where the drawing data begins.

Returns: undefined 

Content timeline steps: 

Issue the subsequent steps on the Device timeline of this . [[device]] . 

Device timeline steps:

Validate the encoder state of this . If it returns false, return. 

If any of the following conditions are unsatisfied, invalidate this and return. 

It is valid to draw indexed with this . 

indirectBuffer is valid to use with this . 

indirectBuffer . usage contains INDIRECT . 

indirectOffset + sizeof( indirect drawIndexed parameters ) ≤
indirectBuffer . size . 

indirectOffset is a multiple of 4. 

Add indirectBuffer to [[usage scope]] 
with usage input . 

Increment this . [[drawCount]] by 1. 

Let bindingState be a snapshot of this ’s current state. 

Enqueue a render command on this which issues the subsequent steps on the
Queue timeline with renderState when executed. 

Queue timeline steps:

Let indexCount be an unsigned 32-bit integer read from indirectBuffer at
indirectOffset bytes. 

Let instanceCount be an unsigned 32-bit integer read from indirectBuffer at
( indirectOffset + 4) bytes. 

Let firstIndex be an unsigned 32-bit integer read from indirectBuffer at
( indirectOffset + 8) bytes. 

Let baseVertex be a signed 32-bit integer read from indirectBuffer at
( indirectOffset + 12) bytes. 

Let firstInstance be an unsigned 32-bit integer read from indirectBuffer at
( indirectOffset + 16) bytes. 

Draw instanceCount instances, starting with instance firstInstance , of
primitives consisting of indexCount indexed vertices, starting with index
firstIndex from vertex baseVertex ,
with the states from bindingState and renderState . 

To determine if it’s valid to draw with GPURenderCommandsMixin encoder ,
run the following device timeline steps:

If any of the following conditions are unsatisfied, return false : 

Validate encoder bind groups ( encoder , encoder . [[pipeline]] )
must be true . 

Let pipelineDescriptor be encoder . [[pipeline]] . [[descriptor]] . 

For each GPUIndex32 slot 0 to
pipelineDescriptor . vertex . buffers . size : 

If pipelineDescriptor . vertex . buffers [ slot ] is not null ,
encoder . [[vertex_buffers]] must contain slot . 

Validate maxBindGroupsPlusVertexBuffers : 

Let bindGroupSpaceUsed be
(the maximum key in encoder . [[bind_groups]] ) + 1. 

Let vertexBufferSpaceUsed be
(the maximum key in encoder . [[vertex_buffers]] ) + 1. 

bindGroupSpaceUsed + vertexBufferSpaceUsed must be ≤
encoder . [[device]] . [[limits]] . maxBindGroupsPlusVertexBuffers . 

Otherwise, return true . 

To determine if it’s valid to draw indexed with GPURenderCommandsMixin encoder ,
run the following device timeline steps:

If any of the following conditions are unsatisfied, return false : 

It must be valid to draw with encoder . 

encoder . [[index_buffer]] must not be null . 

Let topology be encoder . [[pipeline]] . [[descriptor]] . primitive . topology . 

If topology is "line-strip" or "triangle-strip" : 

encoder . [[index_format]] must equal
encoder . [[pipeline]] . [[descriptor]] . primitive . stripIndexFormat . 

Otherwise, return true . 

17.2.2. Rasterization state 

The GPURenderPassEncoder has several methods which affect how draw commands are rasterized to
attachments used by this encoder. 

setViewport(x, y, width, height, minDepth, maxDepth) 

Sets the viewport used during the rasterization stage to linearly map from
normalized device coordinates to viewport coordinates . 

Called on: GPURenderPassEncoder this .

Arguments: 

Arguments for the GPURenderPassEncoder.setViewport(x, y, width, height, minDepth, maxDepth) method. 

Parameter
Type
Nullable
Optional
Description

x 
float 
✘ 
✘ 
Minimum X value of the viewport in pixels.

y 
float 
✘ 
✘ 
Minimum Y value of the viewport in pixels.

width 
float 
✘ 
✘ 
Width of the viewport in pixels.

height 
float 
✘ 
✘ 
Height of the viewport in pixels.

minDepth 
float 
✘ 
✘ 
Minimum depth value of the viewport.

maxDepth 
float 
✘ 
✘ 
Maximum depth value of the viewport.

Returns: undefined 

Content timeline steps: 

Issue the subsequent steps on the Device timeline of
this . [[device]] . 

Device timeline steps:

Validate the encoder state of this . If it returns false, return. 

Let maxViewportRange be this . limits . maxTextureDimension2D × 2 . 

If any of the following conditions are unsatisfied, invalidate this and return. 

x ≥ - maxViewportRange 

y ≥ - maxViewportRange 

0 ≤ width ≤ this . limits . maxTextureDimension2D 

0 ≤ height ≤ this . limits . maxTextureDimension2D 

x + width ≤ maxViewportRange − 1 

y + height ≤ maxViewportRange − 1 

0.0 ≤ minDepth ≤ 1.0 

0.0 ≤ maxDepth ≤ 1.0 

minDepth ≤ maxDepth 

Enqueue a render command on this which issues the subsequent steps on the
Queue timeline with renderState when executed. 

Queue timeline steps:

Round x , y , width , and height to some uniform precision, no less precise than integer rounding. 

Set renderState . [[viewport]] to the extents x , y , width , height , minDepth , and maxDepth . 

setScissorRect(x, y, width, height) 

Sets the scissor rectangle used during the rasterization stage.
After transformation into viewport coordinates any fragments which fall outside the scissor
rectangle will be discarded. 

Called on: GPURenderPassEncoder this .

Arguments: 

Arguments for the GPURenderPassEncoder.setScissorRect(x, y, width, height) method. 

Parameter
Type
Nullable
Optional
Description

x 
GPUIntegerCoordinate 
✘ 
✘ 
Minimum X value of the scissor rectangle in pixels.

y 
GPUIntegerCoordinate 
✘ 
✘ 
Minimum Y value of the scissor rectangle in pixels.

width 
GPUIntegerCoordinate 
✘ 
✘ 
Width of the scissor rectangle in pixels.

height 
GPUIntegerCoordinate 
✘ 
✘ 
Height of the scissor rectangle in pixels.

Returns: undefined 

Content timeline steps: 

Issue the subsequent steps on the Device timeline of
this . [[device]] . 

Device timeline steps:

Validate the encoder state of this . If it returns false, return. 

If any of the following conditions are unsatisfied, invalidate this and return. 

x + width ≤
this . [[attachment_size]] .width. 

y + height ≤
this . [[attachment_size]] .height. 

Enqueue a render command on this which issues the subsequent steps on the
Queue timeline with renderState when executed. 

Queue timeline steps:

Set renderState . [[scissorRect]] to the extents x , y , width , and height . 

setBlendConstant(color) 

Sets the constant blend color and alpha values used with "constant" 
and "one-minus-constant" GPUBlendFactor s. 

Called on: GPURenderPassEncoder this.

Arguments: 

Arguments for the GPURenderPassEncoder.setBlendConstant(color) method. 

Parameter
Type
Nullable
Optional
Description

color 
GPUColor 
✘ 
✘ 
The color to use when blending.

Returns: undefined 

Content timeline steps: 

? validate GPUColor shape ( color ). 

Issue the subsequent steps on the Device timeline of
this . [[device]] . 

Device timeline steps:

Validate the encoder state of this . If it returns false, return. 

Enqueue a render command on this which issues the subsequent steps on the
Queue timeline with renderState when executed. 

Queue timeline steps:

Set renderState . [[blendConstant]] to color . 

setStencilReference(reference) 

Sets the [[stencilReference]] value used during stencil tests with
the "replace" GPUStencilOperation . 

Called on: GPURenderPassEncoder this.

Arguments: 

Arguments for the GPURenderPassEncoder.setStencilReference(reference) method. 

Parameter
Type
Nullable
Optional
Description

reference 
GPUStencilValue 
✘ 
✘ 
The new stencil reference value.

Returns: undefined 

Content timeline steps: 

Issue the subsequent steps on the Device timeline of
this . [[device]] . 

Device timeline steps:

Validate the encoder state of this . If it returns false, return. 

Enqueue a render command on this which issues the subsequent steps on the
Queue timeline with renderState when executed. 

Queue timeline steps:

Set renderState . [[stencilReference]] to reference . 

17.2.3. Queries 

beginOcclusionQuery(queryIndex) 

Called on: GPURenderPassEncoder this .

Arguments: 

Arguments for the GPURenderPassEncoder.beginOcclusionQuery(queryIndex) method. 

Parameter
Type
Nullable
Optional
Description

queryIndex 
GPUSize32 
✘ 
✘ 
The index of the query in the query set.

Returns: undefined 

Content timeline steps: 

Issue the subsequent steps on the Device timeline of
this . [[device]] . 

Device timeline steps:

Validate the encoder state of this . If it returns false, return. 

If any of the following conditions are unsatisfied, invalidate this and return. 

this . [[occlusion_query_set]] is not null . 

queryIndex < this . [[occlusion_query_set]] . count . 

The query at same queryIndex must not have been previously written to in this pass. 

this . [[occlusion_query_active]] is false . 

Set this . [[occlusion_query_active]] to true . 

Enqueue a render command on this which issues the subsequent steps on the
Queue timeline with renderState when executed. 

Queue timeline steps:

Set renderState . [[occlusionQueryIndex]] to queryIndex . 

endOcclusionQuery() 

Called on: GPURenderPassEncoder this.

Returns: undefined 

Content timeline steps: 

Issue the subsequent steps on the Device timeline of
this . [[device]] . 

Device timeline steps:

Validate the encoder state of this . If it returns false, return. 

If any of the following conditions are unsatisfied, invalidate this and return. 

this . [[occlusion_query_active]] is true . 

Set this . [[occlusion_query_active]] to false . 

Enqueue a render command on this which issues the subsequent steps on the
Queue timeline with renderState when executed. 

Queue timeline steps:

Let passingFragments be non-zero if any fragment samples passed all per-fragment
tests since the corresponding beginOcclusionQuery() 
command was executed, and zero otherwise. 

Note: If no draw calls occurred, passingFragments is zero. 

Write passingFragments into
this . [[occlusion_query_set]] at index
renderState . [[occlusionQueryIndex]] . 

17.2.4. Bundles 

executeBundles(bundles) 

Executes the commands previously recorded into the given GPURenderBundle s as part of
this render pass. 

When a GPURenderBundle is executed, it does not inherit the render pass’s pipeline, bind
groups, or vertex and index buffers. After a GPURenderBundle has executed, the render
pass’s pipeline, bind group, and vertex/index buffer state is cleared
(to the initial, empty values). 

Note: The state is cleared, not restored to the previous state.
This occurs even if zero GPURenderBundles are executed. 

Called on: GPURenderPassEncoder this.

Arguments: 

Arguments for the GPURenderPassEncoder.executeBundles(bundles) method. 

Parameter
Type
Nullable
Optional
Description

bundles 
sequence < GPURenderBundle > 
✘ 
✘ 
List of render bundles to execute.

Returns: undefined 

Content timeline steps: 

Issue the subsequent steps on the Device timeline of
this . [[device]] . 

Device timeline steps:

Validate the encoder state of this . If it returns false, return. 

If any of the following conditions are unsatisfied, invalidate this and return. 

For each bundle in bundles : 

bundle must be valid to use with this . 

this . [[layout]] must equal bundle . [[layout]] . 

If this . [[depthReadOnly]] is true, bundle . [[depthReadOnly]] must be true. 

If this . [[stencilReadOnly]] is true, bundle . [[stencilReadOnly]] must be true. 

For each bundle in bundles : 

Increment this . [[drawCount]] by bundle . [[drawCount]] . 

Merge bundle . [[usage scope]] into
this . [[usage scope]] . 

Extend this . [[used_bind_groups]] with bundle . [[used_bind_groups]] 

Enqueue a render command on this which issues the following steps on the
Queue timeline with renderState when executed: 

Queue timeline steps:

Execute each command in bundle . [[command_list]] 
with renderState . 

Note: renderState cannot be changed by executing render bundles. Binding state was
already captured at bundle encoding time, and so isn’t used when executing bundles. 

Reset the render pass binding state of this . 

To Reset the render pass binding state of GPURenderPassEncoder encoder run
the following device timeline steps:

Clear encoder . [[bind_groups]] . 

Set encoder . [[pipeline]] to null . 

Set encoder . [[index_buffer]] to null . 

Clear encoder . [[vertex_buffers]] . 

18. Bundles 

A bundle is a partial, limited pass that is encoded once and can then be executed multiple times as
part of future pass encoders without expiring after use like typical command buffers. This can
reduce the overhead of encoding and submission of commands which are issued repeatedly without
changing. 

18.1. GPURenderBundle 

[ Exposed =( Window , Worker ), SecureContext ]
interface GPURenderBundle {
};
GPURenderBundle includes GPUObjectBase ;

[[command_list]] , of type list < GPU command >

A list of GPU commands to be submitted to the GPURenderPassEncoder when the
GPURenderBundle is executed. 

[[used_bind_groups]] , of type set < GPUBindGroup >, readonly

A set of all GPUBindGroup s used by this render bundle. 

[[usage scope]] , of type usage scope , initially empty

The usage scope for this render bundle, stored for later merging into the
GPURenderPassEncoder ’s [[usage scope]] 
in executeBundles() . 

[[layout]] , of type GPURenderPassLayout 

The layout of the render bundle. 

[[depthReadOnly]] , of type boolean 

If true , indicates that the depth component is not modified by executing this render bundle. 

[[stencilReadOnly]] , of type boolean 

If true , indicates that the stencil component is not modified by executing this render bundle. 

[[drawCount]] , of type GPUSize64 

The number of draw commands in this GPURenderBundle . 

18.1.1. Render Bundle Creation 

dictionary GPURenderBundleDescriptor 
: GPUObjectDescriptorBase {
};

[ Exposed =( Window , Worker ), SecureContext ]
interface GPURenderBundleEncoder {
GPURenderBundle finish ( optional GPURenderBundleDescriptor descriptor = {});
};
GPURenderBundleEncoder includes GPUObjectBase ;
GPURenderBundleEncoder includes GPUCommandsMixin ;
GPURenderBundleEncoder includes GPUDebugCommandsMixin ;
GPURenderBundleEncoder includes GPUBindingCommandsMixin ;
GPURenderBundleEncoder includes GPURenderCommandsMixin ;

createRenderBundleEncoder(descriptor) 

Creates a GPURenderBundleEncoder . 

Called on: GPUDevice this .

Arguments: 

Arguments for the GPUDevice.createRenderBundleEncoder(descriptor) method. 

Parameter
Type
Nullable
Optional
Description

descriptor 
GPURenderBundleEncoderDescriptor 
✘ 
✘ 
Description of the GPURenderBundleEncoder to create.

Returns: GPURenderBundleEncoder 

Content timeline steps: 

? Validate texture format required features of each non- null element of
descriptor . colorFormats with this . [[device]] . 

If descriptor . depthStencilFormat is provided : 

? Validate texture format required features of
descriptor . depthStencilFormat with this . [[device]] . 

Let e be ! create a new WebGPU object ( this , GPURenderBundleEncoder , descriptor ). 

Issue the initialization steps on the Device timeline of this . 

Return e . 

Device timeline initialization steps :

If any of the following conditions are unsatisfied
generate a validation error , invalidate e and return. 

this must not be lost . 

descriptor . colorFormats . size must be ≤
this . [[limits]] . maxColorAttachments . 

For each non- null colorFormat in descriptor . colorFormats : 

colorFormat must be a color renderable format . 

Calculating color attachment bytes per sample ( descriptor . colorFormats )
must be ≤ this . [[limits]] . maxColorAttachmentBytesPerSample . 

If descriptor . depthStencilFormat is provided : 

descriptor . depthStencilFormat must be a
depth-or-stencil format . 

There must exist at least one attachment, either: 

A non- null value in
descriptor . colorFormats , or 

A descriptor . depthStencilFormat . 

Set e . [[layout]] to a copy of descriptor ’s included GPURenderPassLayout interface. 

Set e . [[depthReadOnly]] to descriptor . depthReadOnly . 

Set e . [[stencilReadOnly]] to descriptor . stencilReadOnly . 

Set e . [[state]] to " open ". 

Set e . [[drawCount]] to 0. 

18.1.2. Encoding 

dictionary GPURenderBundleEncoderDescriptor 
: GPURenderPassLayout {
boolean depthReadOnly = false ;
boolean stencilReadOnly = false ;
};

depthReadOnly , of type boolean , defaulting to false 

If true , indicates that the render bundle does not modify the depth component of the
GPURenderPassDepthStencilAttachment of any render pass the render bundle is executed
in. 

See read-only depth-stencil . 

stencilReadOnly , of type boolean , defaulting to false 

If true , indicates that the render bundle does not modify the stencil component of the
GPURenderPassDepthStencilAttachment of any render pass the render bundle is executed
in. 

See read-only depth-stencil . 

18.1.3. Finalization 

finish(descriptor) 

Completes recording of the render bundle commands sequence. 

Called on: GPURenderBundleEncoder this.

Arguments: 

Arguments for the GPURenderBundleEncoder.finish(descriptor) method. 

Parameter
Type
Nullable
Optional
Description

descriptor 
GPURenderBundleDescriptor 
✘ 
✔ 

Returns: GPURenderBundle 

Content timeline steps: 

Let renderBundle be a new GPURenderBundle . 

Issue the finish steps on the Device timeline of
this . [[device]] . 

Return renderBundle . 

Device timeline finish steps :

Let validationSucceeded be true if all of the following requirements are met, and false otherwise. 

this must be valid . 

this . [[usage scope]] must satisfy usage scope validation . 

this . [[state]] must be " open ". 

this . [[debug_group_stack]] must be empty . 

Set this . [[state]] to " ended ". 

If validationSucceeded is false , then: 

Generate a validation error . 

Return an invalidated GPURenderBundle . 

Set renderBundle . [[command_list]] to
this . [[commands]] . 

Set renderBundle . [[used_bind_groups]] to
this . [[used_bind_groups]] . 

Set renderBundle . [[usage scope]] to
this . [[usage scope]] . 

Set renderBundle . [[drawCount]] to
this . [[drawCount]] . 

19. Queues 

19.1. GPUQueueDescriptor 

GPUQueueDescriptor describes a queue request. 

dictionary GPUQueueDescriptor 
: GPUObjectDescriptorBase {
};

19.2. GPUQueue 

[ Exposed =( Window , Worker ), SecureContext ]
interface GPUQueue {
undefined submit ( sequence < GPUCommandBuffer > commandBuffers );

Promise < undefined > onSubmittedWorkDone ();

undefined writeBuffer (
GPUBuffer buffer ,
GPUSize64 bufferOffset ,
AllowSharedBufferSource data ,
optional GPUSize64 dataOffset = 0,
optional GPUSize64 size );

undefined writeTexture (
GPUTexelCopyTextureInfo destination ,
AllowSharedBufferSource data ,
GPUTexelCopyBufferLayout dataLayout ,
GPUExtent3D size );

undefined copyExternalImageToTexture (
GPUCopyExternalImageSourceInfo source ,
GPUCopyExternalImageDestInfo destination ,
GPUExtent3D copySize );
};
GPUQueue includes GPUObjectBase ;

GPUQueue has the following methods: 

writeBuffer(buffer, bufferOffset, data, dataOffset, size) 

Issues a write operation of the provided data into a GPUBuffer . 

Called on: GPUQueue this .

Arguments: 

Arguments for the GPUQueue.writeBuffer(buffer, bufferOffset, data, dataOffset, size) method. 

Parameter
Type
Nullable
Optional
Description

buffer 
GPUBuffer 
✘ 
✘ 
The buffer to write to.

bufferOffset 
GPUSize64 
✘ 
✘ 
Offset in bytes into buffer to begin writing at.

data 
AllowSharedBufferSource 
✘ 
✘ 
Data to write into buffer .

dataOffset 
GPUSize64 
✘ 
✔ 
Offset in into data to begin writing from. Given in elements if
data is a TypedArray and bytes otherwise.

size 
GPUSize64 
✘ 
✔ 
Size of content to write from data to buffer . Given in elements if
data is a TypedArray and bytes otherwise.

Returns: undefined 

Content timeline steps: 

If data is an ArrayBuffer or DataView , let the element type be "byte".
Otherwise, data is a TypedArray; let the element type be the type of the TypedArray. 

Let dataSize be the size of data , in elements. 

If size is missing,
let contentsSize be dataSize − dataOffset .
Otherwise, let contentsSize be size . 

If any of the following conditions are unsatisfied,
throw an OperationError and return. 

contentsSize ≥ 0. 

dataOffset + contentsSize ≤ dataSize . 

contentsSize , converted to bytes, is a multiple of 4 bytes. 

Let dataContents be a copy of the bytes held by the buffer source data . 

Let contents be the contentsSize elements of dataContents starting at
an offset of dataOffset elements. 

Issue the subsequent steps on the Device timeline of this . 

Device timeline steps:

If any of the following conditions are unsatisfied,
generate a validation error and return. 

buffer is valid to use with this . 

buffer . [[internal state]] is " available ". 

buffer . usage includes COPY_DST . 

bufferOffset , converted to bytes, is a multiple of 4 bytes. 

bufferOffset + contentsSize , converted to bytes, ≤ buffer . size bytes. 

Issue the subsequent steps on the Queue timeline of this . 

Queue timeline steps:

Write contents into buffer starting at bufferOffset . 

writeTexture(destination, data, dataLayout, size) 

Issues a write operation of the provided data into a GPUTexture . 

Called on: GPUQueue this .

Arguments: 

Arguments for the GPUQueue.writeTexture(destination, data, dataLayout, size) method. 

Parameter
Type
Nullable
Optional
Description

destination 
GPUTexelCopyTextureInfo 
✘ 
✘ 
The texture subresource and origin to write to.

data 
AllowSharedBufferSource 
✘ 
✘ 
Data to write into destination .

dataLayout 
GPUTexelCopyBufferLayout 
✘ 
✘ 
Layout of the content in data .

size 
GPUExtent3D 
✘ 
✘ 
Extents of the content to write from data to destination .

Returns: undefined 

Content timeline steps: 

? validate GPUOrigin3D shape ( destination . origin ). 

? validate GPUExtent3D shape ( size ). 

Let dataBytes be a copy of the bytes held by the buffer source data . 

Note: This is described as copying all of data to the device timeline,
but in practice data could be much larger than necessary.
Implementations should optimize by copying only the necessary bytes. 

Issue the subsequent steps on the Device timeline of this . 

Device timeline steps:

Let aligned be false . 

Let dataLength be dataBytes . length . 

If any of the following conditions are unsatisfied,
generate a validation error and return. 

destination . texture . [[destroyed]] is false . 

validating texture buffer copy ( destination , dataLayout , dataLength , size , COPY_DST , aligned ) returns true . 

Note: unlike
GPUCommandEncoder . copyBufferToTexture() ,
there is no alignment requirement on either
dataLayout . bytesPerRow or dataLayout . offset . 

Issue the subsequent steps on the Queue timeline of this . 

Queue timeline steps:

Let blockWidth be the texel block width of destination . texture . 

Let blockHeight be the texel block height of destination . texture . 

Let dstOrigin be destination . origin ; 

Let dstBlockOriginX be ( dstOrigin . x ÷ blockWidth ). 

Let dstBlockOriginY be ( dstOrigin . y ÷ blockHeight ). 

Let blockColumns be ( copySize . width ÷ blockWidth ). 

Let blockRows be ( copySize . height ÷ blockHeight ). 

Assert that dstBlockOriginX , dstBlockOriginY , blockColumns , and blockRows are integers. 

For each z in the range [0, copySize . depthOrArrayLayers − 1]: 

Let dstSubregion be texture copy sub-region ( z + dstOrigin . z ) of destination . 

For each y in the range [0, blockRows − 1]: 

For each x in the range [0, blockColumns − 1]: 

Let blockOffset be the texel block byte offset of dataLayout for ( x , y , z ) of
destination . texture . 

Set texel block ( dstBlockOriginX + x , dstBlockOriginY + y ) of
dstSubregion to be an equivalent texel representation to the texel block 
described by dataBytes at offset blockOffset . 

copyExternalImageToTexture(source, destination, copySize) 

Issues a copy operation of the contents of a platform image/canvas
into the destination texture. 

This operation performs color encoding into the destination
encoding according to the parameters of GPUCopyExternalImageDestInfo . 

Copying into a -srgb texture results in the same texture bytes, not the same decoded
values, as copying into the corresponding non- -srgb format.
Thus, after a copy operation, sampling the destination texture has
different results depending on whether its format is -srgb , all else unchanged. 

NOTE: 

When copying from a "webgl" / "webgl2" context canvas, the
WebGL Drawing Buffer may be not exist during certain points in the
frame presentation cycle (after the image has been moved to the compositor
for display). To avoid this, either:

Issue copyExternalImageToTexture() in the same task with
WebGL rendering operation, to ensure the copy occurs before the WebGL
canvas is presented. 

If not possible, set the preserveDrawingBuffer option in
WebGLContextAttributes to true , so that the drawing buffer will
still contain a copy of the frame contents after they’ve been presented.
Note, this extra copy may have a performance cost. 

Called on: GPUQueue this .

Arguments: 

Arguments for the GPUQueue.copyExternalImageToTexture(source, destination, copySize) method. 

Parameter
Type
Nullable
Optional
Description

source 
GPUCopyExternalImageSourceInfo 
✘ 
✘ 
source image and origin to copy to destination .

destination 
GPUCopyExternalImageDestInfo 
✘ 
✘ 
The texture subresource and origin to write to, and its encoding metadata.

copySize 
GPUExtent3D 
✘ 
✘ 
Extents of the content to write from source to destination .

Returns: undefined 

Content timeline steps: 

? validate GPUOrigin2D shape ( source . origin ). 

? validate GPUOrigin3D shape ( destination . origin ). 

? validate GPUExtent3D shape ( copySize ). 

Let sourceImage be source . source 

If sourceImage is not origin-clean ,
throw a SecurityError and return. 

If any of the following requirements are unmet, throw an OperationError and return. 

source . origin . x + copySize . width 
must be ≤ the width of sourceImage . 

source . origin . y + copySize . height 
must be ≤ the height of sourceImage . 

copySize . depthOrArrayLayers 
must be ≤ 1. 

Let usability be ? check the usability of the image argument ( source ). 

Issue the subsequent steps on the Device timeline of this . 

Device timeline steps:

Let texture be destination . texture . 

If any of the following requirements are unmet, generate a validation error and return. 

usability must be good . 

texture . [[destroyed]] must be false . 

texture must be valid to use with this . 

validating GPUTexelCopyTextureInfo (destination, copySize) must return true . 

texture . usage must include both
RENDER_ATTACHMENT and COPY_DST . 

texture . dimension must be "2d" . 

texture . sampleCount must be 1. 

texture . format must be a plain color format 
supporting RENDER_ATTACHMENT and be a
unorm / unorm-srgb or float / ufloat format (not snorm , uint , or sint ). 

If copySize . depthOrArrayLayers is > 0, issue the subsequent
steps on the Queue timeline of this . 

Queue timeline steps:

Assert that the texel block width of destination . texture is 1,
the texel block height of destination . texture is 1, and that
copySize . depthOrArrayLayers is 1. 

Let srcOrigin be source . origin . 

Let dstOrigin be destination . origin . 

Let dstSubregion be texture copy sub-region ( dstOrigin . z ) of destination . 

For each y in the range [0, copySize . height − 1]: 

Let srcY be y if source . flipY is false and
( copySize . height − 1 − y ) otherwise. 

For each x in the range [0, copySize . width − 1]: 

Let srcColor be the color-managed color value of the pixel at
( srcOrigin . x + x , srcOrigin . y + srcY ) of
source . source . 

Let dstColor be the numeric RGBA value resulting from applying any
color encoding required by
destination . colorSpace and
destination . premultipliedAlpha 
to srcColor . 

If texture . format is an -srgb format: 

Set dstColor to the result of applying the sRGB non-linear-to-linear conversion to it. 

Note: 
This cancels out the sRGB linear-to-non-linear conversion that occurs
when writing an -srgb format in the next step, so that precision
from an sRGB-like input image is not lost and the linear color values
of the original image can be read from the texture
(as is generally the purpose of using -srgb formats). 

Set texel block 
( dstOrigin . x + x , dstOrigin . y + y ) of
dstSubregion to an equivalent texel representation of dstColor . 

submit(commandBuffers) 

Schedules the execution of the command buffers by the GPU on this queue. 

Submitted command buffers cannot be used again. 

Called on: GPUQueue this.

Arguments: 

Arguments for the GPUQueue.submit(commandBuffers) method. 

Parameter
Type
Nullable
Optional
Description

commandBuffers 
sequence < GPUCommandBuffer > 
✘ 
✘ 

Returns: undefined 

Content timeline steps: 

Issue the subsequent steps on the Device timeline of this : 

Device timeline steps:

If any of the following requirements are unmet, generate a validation error ,
invalidate each GPUCommandBuffer in commandBuffers and return. 

Every GPUCommandBuffer in commandBuffers must be unique. 

For each commandBuffer in commandBuffers : 

commandBuffer must be valid to use with this 

For each bindGroup in commandBuffer . [[used_bind_groups]] : 

For each GPUBindingResource in bindGroup , if the resource type is: 

GPUBuffer b 

b . [[internal state]] must
be " available ". 

GPUTexture t 

t . [[destroyed]] must be false . 

GPUExternalTexture et 

et . [[expired]] must be false . 

GPUQuerySet qs 

qs . [[destroyed]] must be false . 

Note: 
For occlusion queries, the occlusionQuerySet 
in beginRenderPass() is not "used" unless
it is also used by beginOcclusionQuery() . 

For each commandBuffer in commandBuffers : 

Invalidate commandBuffer . 

Issue the subsequent steps on the Queue timeline of this : 

Queue timeline steps:

For each commandBuffer in commandBuffers : 

Execute each command in commandBuffer . [[command_list]] . 

onSubmittedWorkDone() 

Returns a Promise that resolves once this queue finishes processing all the work submitted
up to this moment. 

Resolution of this Promise implies the completion of
mapAsync() calls made prior to that call,
on GPUBuffer s last used exclusively on that queue. 

Called on: GPUQueue this .

Returns: Promise < undefined > 

Content timeline steps: 

Let contentTimeline be the current Content timeline . 

Let promise be a new promise . 

Issue the synchronization steps on the Device timeline of this . 

Return promise . 

Device timeline synchronization steps :

Let event occur upon the completion of
all currently-enqueued operations . 

Listen for timeline event event 
on this . [[device]] , handled by
the subsequent steps on contentTimeline . 

Content timeline steps:

Resolve promise . 

20. Queries 

20.1. GPUQuerySet 

[ Exposed =( Window , Worker ), SecureContext ]
interface GPUQuerySet {
undefined destroy ();

readonly attribute GPUQueryType type ;
readonly attribute GPUSize32Out count ;
};
GPUQuerySet includes GPUObjectBase ;

GPUQuerySet has the following immutable properties : 

type , of type GPUQueryType , readonly 

The type of the queries managed by this GPUQuerySet . 

count , of type GPUSize32Out , readonly 

The number of queries managed by this GPUQuerySet . 

GPUQuerySet has the following device timeline properties : 

[[destroyed]] , of type boolean , initially false 

If the query set is destroyed, it can no longer be used in any operation,
and its underlying memory can be freed. 

20.1.1. QuerySet Creation 

A GPUQuerySetDescriptor specifies the options to use in creating a GPUQuerySet . 

dictionary GPUQuerySetDescriptor 
: GPUObjectDescriptorBase {
required GPUQueryType type ;
required GPUSize32 count ;
};

type , of type GPUQueryType 

The type of queries managed by GPUQuerySet . 

count , of type GPUSize32 

The number of queries managed by GPUQuerySet . 

createQuerySet(descriptor) 

Creates a GPUQuerySet . 

Called on: GPUDevice this.

Arguments: 

Arguments for the GPUDevice.createQuerySet(descriptor) method. 

Parameter
Type
Nullable
Optional
Description

descriptor 
GPUQuerySetDescriptor 
✘ 
✘ 
Description of the GPUQuerySet to create.

Returns: GPUQuerySet 

Content timeline steps: 

If descriptor . type is "timestamp" ,
but "timestamp-query" is not enabled for this : 

Throw a TypeError . 

Let q be ! create a new WebGPU object ( this , GPUQuerySet , descriptor ). 

Set q . type to descriptor . type . 

Set q . count to descriptor . count . 

Issue the initialization steps on the Device timeline of this . 

Return q . 

Device timeline initialization steps :

If any of the following requirements are unmet, generate a validation error ,
invalidate q and return. 

this must not be lost . 

descriptor . count must be ≤ 4096. 

Create a device allocation for q where each entry in the query set is zero. 

If the allocation fails without side-effects,
generate an out-of-memory error , invalidate q , and return. 

Creating a GPUQuerySet which holds 32 occlusion query results.

const querySet = gpuDevice . createQuerySet ({ 
type : 'occlusion' , 
count : 32 
}); 

20.1.2. Query Set Destruction 

An application that no longer requires a GPUQuerySet can choose to lose access to it before
garbage collection by calling destroy() . 

GPUQuerySet has the following methods: 

destroy() 

Destroys the GPUQuerySet . 

Called on: GPUQuerySet this .

Returns: undefined 

Content timeline steps: 

Issue the subsequent steps on the device timeline . 

Device timeline steps:

Set this . [[destroyed]] to true . 

20.2. QueryType 

enum GPUQueryType {
"occlusion" ,
"timestamp" ,
};

20.3. Occlusion Query 

Occlusion query is only available on render passes, to query the number of fragment samples that pass
all the per-fragment tests for a set of drawing commands, including scissor, sample mask, alpha to
coverage, stencil, and depth tests. Any non-zero result value for the query indicates that at least
one sample passed the tests and reached the output merging stage of the render pipeline, 0 indicates
that no samples passed the tests. 

When beginning a render pass, GPURenderPassDescriptor . occlusionQuerySet 
must be set to be able to use occlusion queries during the pass. An occlusion query is begun
and ended by calling beginOcclusionQuery() and
endOcclusionQuery() in pairs that cannot be nested, and resolved into a
GPUBuffer as a 64-bit unsigned integer by GPUCommandEncoder . resolveQuerySet() . 

20.4. Timestamp Query 

Timestamp queries allow applications to write timestamps to a GPUQuerySet , using: 

GPUComputePassDescriptor . timestampWrites 

GPURenderPassDescriptor . timestampWrites 

and then resolve timestamp values (in nanoseconds as a 64-bit unsigned integer ) into
a GPUBuffer , using GPUCommandEncoder . resolveQuerySet() . 

Timestamp values are implementation-defined .
Applications must handle arbitrary timestamp results, and should not be written in such a way that unexpected
timestamps cause an application failure. 

Note: 
The physical device may reset the timestamp counter occasionally, which can
result in unexpected values such as negative deltas from one timestamp to the next.
These instances should be rare, and these data points can safely be discarded. 

Timestamp queries are implemented using high-resolution timers (see § 2.1.7.2 Device/queue-timeline timing ).
To mitigate security and privacy concerns, their precision must be reduced:

To get the current queue timestamp , run the following queue timeline steps:

Let fineTimestamp be the current timestamp value of the current queue timeline ,
in nanoseconds, relative to an implementation-defined point in the past. 

Return the result of calling coarsen time on fineTimestamp 
with crossOriginIsolatedCapability set to false . 

Note: Cross-origin isolation never applies to the device timeline or
queue timeline , so crossOriginIsolatedCapability is never set to true . 

Validate timestampWrites ( device , timestampWrites )

Arguments: 

GPUDevice device 

( GPUComputePassTimestampWrites or GPURenderPassTimestampWrites ) timestampWrites 

Device timeline steps: 

Return true if the following requirements are met, and false if not: 

"timestamp-query" must be enabled for device . 

timestampWrites . querySet must be valid to use with device . 

timestampWrites . querySet . type must be "timestamp" . 

Of the write index members in timestampWrites ( beginningOfPassWriteIndex , endOfPassWriteIndex ): 

At least one must be provided . 

Of those which are provided : 

No two may be equal. 

Each must be < timestampWrites . querySet . count . 

21. Canvas Rendering 

21.1. HTMLCanvasElement.getContext() 

A GPUCanvasContext object is created 
via the getContext() method of an HTMLCanvasElement 
instance by passing the string literal 'webgpu' as its contextType argument. 

Get a GPUCanvasContext from an offscreen HTMLCanvasElement :

const canvas = document . createElement ( 'canvas' ); 
const context = canvas . getContext ( 'webgpu' ); 

Unlike WebGL or 2D context creation, the second argument of
HTMLCanvasElement.getContext() or
OffscreenCanvas.getContext() ,
the context creation attribute dictionary options , is ignored.
Instead, use GPUCanvasContext.configure() ,
which allows changing the canvas configuration without replacing the canvas. 

To create a 'webgpu' context on a canvas 
( HTMLCanvasElement or OffscreenCanvas ) canvas , run the following
content timeline steps:

Let context be a new GPUCanvasContext . 

Set context . canvas to canvas . 

Replace the drawing buffer of context . 

Return context . 

Note: User agents should consider issuing developer-visible warnings when
an ignored options argument is provided when calling getContext() 
to get a WebGPU canvas context. 

21.2. GPUCanvasContext 

[ Exposed =( Window , Worker ), SecureContext ]
interface GPUCanvasContext {
readonly attribute ( HTMLCanvasElement or OffscreenCanvas ) canvas ;

undefined configure ( GPUCanvasConfiguration configuration );
undefined unconfigure ();

GPUCanvasConfiguration ? getConfiguration ();
GPUTexture getCurrentTexture ();
};

GPUCanvasContext has the following content timeline properties : 

canvas , of type (HTMLCanvasElement or OffscreenCanvas) , readonly 

The canvas this context was created from. 

[[configuration]] , of type GPUCanvasConfiguration ?, initially null 

The options this context is currently configured with. 

null if the context has not been configured or has been
unconfigured . 

[[textureDescriptor]] , of type GPUTextureDescriptor ?, initially null 

The currently configured texture descriptor, derived from the
[[configuration]] and canvas. 

null if the context has not been configured or has been
unconfigured . 

[[drawingBuffer]] , an image, initially
a transparent black image with the same size as the canvas

The drawing buffer is the working-copy image data of the canvas.
It is exposed as writable by [[currentTexture]] 
(returned by getCurrentTexture() ). 

The drawing buffer is used to get a copy of the image contents of a context , which
occurs when the canvas is displayed or otherwise read. It may be transparent, even if
[[configuration]] . alphaMode is
"opaque" . The alphaMode only affects the
result of the " get a copy of the image contents of a context " algorithm. 

The drawing buffer outlives the [[currentTexture]] and contains the
previously-rendered contents even after the canvas has been presented.
It is only cleared in Replace the drawing buffer . 

Any time the drawing buffer is read, implementations must ensure that all previously
submitted work (e.g. queue submissions) have completed writing to it via
[[currentTexture]] . 

[[currentTexture]] , of type GPUTexture ?, initially null 

The GPUTexture to draw into for the current frame.
It exposes a writable view onto the underlying [[drawingBuffer]] .
getCurrentTexture() populates this slot if null , then returns it. 

In the steady-state of a visible canvas, any changes to the drawing buffer made through the
currentTexture get presented when updating the rendering of a WebGPU canvas .
At or before that point, the texture is also destroyed
and [[currentTexture]] is set to to null , signalling that
a new one is to be created by the next call to getCurrentTexture() . 

Destroying the currentTexture has no effect on the drawing buffer
contents; it only terminates write-access to the drawing buffer early.
During the same frame, getCurrentTexture() continues returning the
same destroyed texture. 

Expire the current texture sets the currentTexture to null .
It is called by configure() , resizing the canvas,
presentation, transferToImageBitmap() , and others. 

[[lastPresentedImage]] , of type (readonly image)? , initially null 

The image most recently presented for this canvas in " updating the rendering of a WebGPU canvas ".
If the device is lost or destroyed, this image may be used as a fallback in
" get a copy of the image contents of a context " in order to prevent the canvas from going blank. 

Note: 
This property only needs to exist in implementations which implement the fallback, which is optional. 

GPUCanvasContext has the following methods: 

configure(configuration) 

Configures the context for this canvas.
This clears the drawing buffer to transparent black (in Replace the drawing buffer ). 

See getConfiguration() for information on feature detection . 

Called on: GPUCanvasContext this .

Arguments: 

Arguments for the GPUCanvasContext.configure(configuration) method. 

Parameter
Type
Nullable
Optional
Description

configuration 
GPUCanvasConfiguration 
✘ 
✘ 
Desired configuration for the context.

Returns: undefined 

Content timeline steps: 

Let device be configuration . device . 

? Validate texture format required features of
configuration . format with device . [[device]] . 

? Validate texture format required features of each element of
configuration . viewFormats with device . [[device]] . 

If Supported context formats does not contain 
configuration . format , throw a TypeError . 

If configuration . usage includes the
TRANSIENT_ATTACHMENT bit, throw a TypeError . 

Let descriptor be the
GPUTextureDescriptor for the canvas and configuration ( this . canvas , configuration ). 

Set this . [[configuration]] to configuration . 

Note: 
This exposes only the members defined in an implementation’s definition of
GPUCanvasConfiguration . See the specifications of those members for notes about
feature detection . 

Set this . [[textureDescriptor]] to descriptor . 

Replace the drawing buffer of this . 

Issue the subsequent steps on the Device timeline of device . 

Device timeline steps:

If any of the following requirements are unmet, generate a validation error and return. 

validating GPUTextureDescriptor ( device , descriptor )
must return true. 

Note: This early validation remains valid until the next
configure() call, except for
validation of the size , which changes when
the canvas is resized. 

unconfigure() 

Removes the context configuration. Destroys any textures produced while configured. 

Called on: GPUCanvasContext this .

Returns: undefined 

Content timeline steps: 

Set this . [[configuration]] to null . 

Set this . [[textureDescriptor]] to null . 

Replace the drawing buffer of this . 

getConfiguration() 

Returns the context configuration, or null if the context is not configured. 

Note: 
This method exists primarily for feature detection of members (and sub-members) of
GPUCanvasConfiguration ; see those members for details.
For supported members, it returns the originally-supplied values. 

Called on: GPUCanvasContext this .

Returns: GPUCanvasConfiguration or null 

Content timeline steps: 

Let configuration be a copy of this . [[configuration]] . 

Return configuration . 

getCurrentTexture() 

Get the GPUTexture that will be composited to the document by the GPUCanvasContext 
next. 

NOTE: 

An application should call getCurrentTexture() 
in the same task that renders to the canvas texture.
Otherwise, the texture could get destroyed by these steps before the
application is finished rendering to it.

The expiry task (defined below) is optional to implement.
Even if implemented, task source priority is not normatively defined, so may happen as
early as the next task, or as late as after all other task sources are empty
(see automatic expiry task source ).
Expiry is only guaranteed when a visible canvas is displayed
( updating the rendering of a WebGPU canvas ) and in other
callers of " Expire the current texture ". 

Called on: GPUCanvasContext this .

Returns: GPUTexture 

Content timeline steps: 

If this . [[configuration]] is null ,
throw an InvalidStateError and return. 

Assert this . [[textureDescriptor]] is not null . 

Let device be this . [[configuration]] . device . 

If this . [[currentTexture]] is null : 

Replace the drawing buffer of this . 

Set this . [[currentTexture]] to the result of calling
device . createTexture() with this . [[textureDescriptor]] ,
except with the GPUTexture ’s underlying storage pointing to
this . [[drawingBuffer]] . 

Note: 
If the texture can’t be created (e.g. due to validation failure or out-of-memory),
this generates and error and returns an invalidated GPUTexture .
Some validation here is redundant with that done in configure() .
Implementations must not skip this redundant validation. 

Optionally , queue an automatic expiry task with device device and the following steps: 

Expire the current texture of this . 

Note: If this already happened when
updating the rendering of a WebGPU canvas , it has no effect. 

Return this . [[currentTexture]] . 

Note: The same GPUTexture object will be returned by every
call to getCurrentTexture() until " Expire the current texture "
runs, even if that GPUTexture is destroyed, failed validation, or failed to allocate. 

To get a copy of the image contents of a context :

Arguments: 

context : the GPUCanvasContext 

Returns: image contents 

Content timeline steps: 

Let snapshot be a transparent black image of the same size as context . canvas . 

Let configuration be context . [[configuration]] . 

If configuration is null : 

Return snapshot . 

Note: The configuration will be null if the context has not been
configured or has been unconfigured . This is identical to
the behavior when the canvas has no context. 

Ensure that all submitted work items (e.g. queue submissions) have
completed writing to the image (via context . [[currentTexture]] ). 

If configuration . device is found to be valid : 

Set snapshot to a copy of the context . [[drawingBuffer]] . 

Otherwise, if context . [[lastPresentedImage]] is not null : 

Optionally , set snapshot to a copy of context . [[lastPresentedImage]] . 

Note: 
This is optional because the [[lastPresentedImage]] may no longer exist,
depending on what caused device loss.
Implementations may choose to skip it even if do they still have access to that image. 

Let alphaMode be configuration . alphaMode . 

If alphaMode is "opaque" : 

Clear the alpha channel of snapshot to 1.0. 

Note: 
If the [[currentTexture]] , if any, has been destroyed
(for example in " Expire the current texture "), the alpha channel is unobservable,
and implementations may clear the alpha channel in-place. 

Tag snapshot as being opaque. 

Otherwise: 

Tag snapshot with alphaMode . 

Tag snapshot with the colorSpace and
toneMapping of configuration . 

Return snapshot . 

To Replace the drawing buffer of a GPUCanvasContext context , run
the following content timeline steps:

Expire the current texture of context . 

Let configuration be context . [[configuration]] . 

Set context . [[drawingBuffer]] to a transparent black image of the same
size as context . canvas . 

If configuration is null, the drawing buffer is tagged with the color space
"srgb" .
In this case, the drawing buffer will remain blank until the context is configured. 

If not, the drawing buffer has the specified
configuration . format and is tagged with the specified
configuration . colorSpace and
configuration . toneMapping . 

Note: configuration . alphaMode is ignored until
" get a copy of the image contents of a context ". 

NOTE: 

A newly replaced drawing buffer image behaves as if it is cleared to transparent black,
but, like after "discard" , an implementation can clear it lazily only
if it becomes necessary.

Note: This will often be a no-op, if the drawing buffer is already cleared
and has the correct configuration. 

To Expire the current texture of a GPUCanvasContext context , run
the following content timeline steps:

If context . [[currentTexture]] is not null : 

Call context . [[currentTexture]] . destroy() 
(without destroying context . [[drawingBuffer]] )
to terminate write access to the image. 

Set context . [[currentTexture]] to null . 

21.3. HTML Specification Hooks 

The following algorithms "hook" into algorithms in the HTML specification, and must run at the
specified points. 

When the "bitmap" is read from an HTMLCanvasElement or OffscreenCanvas with a
GPUCanvasContext context , run the following content timeline steps:

Return a copy of the image contents 
of context . 

NOTE: 

This occurs in many places, including:

When an HTMLCanvasElement has its rendering updated. 

Including when the canvas is the placeholder canvas element of an OffscreenCanvas . 

When transferToImageBitmap() creates an ImageBitmap from the bitmap.
(See also transferToImageBitmap from WebGPU .) 

When WebGPU canvas contents are read using other Web APIs, like
drawImage() , texImage2D() , texSubImage2D() ,
toDataURL() , toBlob() , and so on. 

If alphaMode is "opaque" ,
this incurs a clear of the alpha channel. Implementations may skip this step when
they are able to read or display images in a way that ignores the alpha channel. 

If an application needs a canvas only for interop (not presentation), avoid
"opaque" if it is not needed. 

When updating the rendering of a WebGPU canvas 
(an HTMLCanvasElement or an OffscreenCanvas with a placeholder canvas element )
with a GPUCanvasContext context , which occurs before getting the canvas’s image contents,
in the following sub-steps of the event loop processing model :

"update the rendering or user interface of that Document " 

"update the rendering of that dedicated worker" 

Note: 
Service and Shared workers do not have "update the rendering" steps
because they cannot render to user-visible canvases.
requestAnimationFrame() is not exposed in
ServiceWorkerGlobalScope and SharedWorkerGlobalScope , and
OffscreenCanvas es from transferControlToOffscreen() 
cannot be sent to these workers . 

Run the following content timeline steps: 

Expire the current texture of context . 

Note: If this already happened in the task queued by
getCurrentTexture() , it has no effect. 

Set context . [[lastPresentedImage]] to
context . [[drawingBuffer]] . 

Note: This is just a reference, not a copy; the drawing buffer’s contents can’t change
in-place after the current texture has expired. 

Note: 
This does not happen for standalone OffscreenCanvas es (created by new OffscreenCanvas() ). 

transferToImageBitmap from WebGPU :

When transferToImageBitmap() is called on a canvas with
GPUCanvasContext context , after creating an ImageBitmap from the canvas’s bitmap,
run the following content timeline steps: 

Replace the drawing buffer of context . 

Note: This makes transferToImageBitmap() 
equivalent to "moving" (and possibly alpha-clearing) the image contents into the
ImageBitmap, without a copy. 

The update the canvas size algorithm. 

21.4. GPUCanvasConfiguration 

The supported context formats are the set of GPUTextureFormat s:
« "bgra8unorm" , "rgba8unorm" ,
"rgba16float" ». These formats must be supported when specified as a
GPUCanvasConfiguration . format regardless of the given
GPUCanvasConfiguration . device . 

Note: Canvas configuration cannot use srgb formats like "bgra8unorm-srgb" .
Instead, use the non- srgb equivalent ( "bgra8unorm" ), specify the srgb 
format in the viewFormats , and use createView() to create
a view with an srgb format. 

enum GPUCanvasAlphaMode {
"opaque" ,
"premultiplied" ,
};

enum GPUCanvasToneMappingMode {
"standard" ,
"extended" ,
};

dictionary GPUCanvasToneMapping {
GPUCanvasToneMappingMode mode = "standard";
};

dictionary GPUCanvasConfiguration {
required GPUDevice device ;
required GPUTextureFormat format ;
GPUTextureUsageFlags usage = 0x10; // GPUTextureUsage.RENDER_ATTACHMENT
sequence < GPUTextureFormat > " href="#dom-gpucanvasconfiguration-viewformats" id="ref-for-dom-gpucanvasconfiguration-viewformats②"> viewFormats = [];
PredefinedColorSpace colorSpace = "srgb";
GPUCanvasToneMapping toneMapping = {};
GPUCanvasAlphaMode alphaMode = "opaque";
};

GPUCanvasConfiguration has the following members: 

device , of type GPUDevice 

The GPUDevice that textures returned by getCurrentTexture() will be
compatible with. 

format , of type GPUTextureFormat 

The format that textures returned by getCurrentTexture() will have.
Must be one of the Supported context formats . 

usage , of type GPUTextureUsageFlags , defaulting to 0x10 

The usage that textures returned by getCurrentTexture() will have.
RENDER_ATTACHMENT is the default, but is not automatically included
if the usage is explicitly set. Be sure to include RENDER_ATTACHMENT 
when setting a custom usage if you wish to use textures returned by
getCurrentTexture() as color targets for a render pass. 

viewFormats , of type sequence< GPUTextureFormat >, defaulting to [] 

The formats that views created from textures returned by
getCurrentTexture() may use. 

colorSpace , of type PredefinedColorSpace , defaulting to "srgb" 

The color space that values written into textures returned by
getCurrentTexture() should be displayed with. 

toneMapping , of type GPUCanvasToneMapping , defaulting to {} 

The tone mapping determines how the content of textures returned by
getCurrentTexture() are to be displayed. 

NOTE: 

This is a required feature, but user agents might not yet implement it,
effectively supporting only the default GPUCanvasToneMapping .
In such implementations, this member should not exist in its implementation of
GPUCanvasConfiguration , to make feature detection possible using
getConfiguration() .

This is especially important in implementations which otherwise have HDR capabilities
(where a dynamic-range of high would be
exposed). 

If an implementation exposes this member and a high dynamic range, it should render the
canvas as an HDR element, not clamp values to the SDR range of the HDR display. 

alphaMode , of type GPUCanvasAlphaMode , defaulting to "opaque" 

Determines the effect that alpha values will have on the content of textures returned by
getCurrentTexture() when read, displayed, or used as an image source. 

Configure a GPUCanvasContext to be used with a specific GPUDevice , using the preferred
format for this context:

const canvas = document . createElement ( 'canvas' ); 
const context = canvas . getContext ( 'webgpu' ); 

context . configure ({ 
device : gpuDevice , 
format : navigator . gpu . getPreferredCanvasFormat (), 
}); 

The GPUTextureDescriptor for the canvas and configuration (
( HTMLCanvasElement or OffscreenCanvas ) canvas ,
GPUCanvasConfiguration configuration )
is a GPUTextureDescriptor with the following members:

size : [ canvas .width, canvas .height, 1]. 

format : configuration . format . 

usage : configuration . usage . 

viewFormats : configuration . viewFormats . 

and other members set to their defaults. 

canvas .width refers to HTMLCanvasElement . width or OffscreenCanvas . width .
canvas .height refers to HTMLCanvasElement . height or OffscreenCanvas . height . 

21.4.1. Canvas Color Space 

During presentation, the color values in the canvas are converted to the color
space of the screen. 

The toneMapping determines the handling of values
outside of the [0, 1] interval in the color space of the screen. 

21.4.2. Canvas Context sizing 

All canvas configuration is set in configure() except for the resolution
of the canvas, which is set by the canvas’s width and height . 

Note: 
Like WebGL and 2d canvas, resizing a WebGPU canvas loses the current contents of the drawing buffer.
In WebGPU, it does so by replacing the drawing buffer . 

When an HTMLCanvasElement or OffscreenCanvas canvas with a
GPUCanvasContext context has its width or height attributes set,
update the canvas size by running the following
content timeline steps:

Replace the drawing buffer of context . 

Let configuration be context . [[configuration]] 

If configuration is not null : 

Set context . [[textureDescriptor]] to the
GPUTextureDescriptor for the canvas and configuration ( canvas , configuration ). 

Note: This may result in a GPUTextureDescriptor which exceeds the
maxTextureDimension2D of the device. In this case,
validation will fail inside getCurrentTexture() . 

Note: This algorithm is run any time the canvas width or height attributes are set, even
if their value is not changed. 

21.5. GPUCanvasToneMappingMode 

This enum specifies how color values are displayed to the screen. 

"standard" 

Color values within the standard dynamic range of the screen are unchanged, and
all other color values are projected to the standard dynamic range of the screen. 

Note: 
This projection is often accomplished by clamping color values in the color space
of the screen to the [0, 1] interval. 

For example, suppose that the value (1.035, -0.175, -0.140) is written to an
'srgb' canvas.

If this is presented to an sRGB screen, then this will be converted to sRGB
(which is a no-op, because the canvas is sRGB), then projected into the display’s space.
Using component-wise clamping, this results in the sRGB value (1.0, 0.0, 0.0) . 

If this is presented to a Display P3 screen, then this will be converted to
the value (0.948, 0.106, 0.01) in the Display P3 color space, and no
clamping will be needed. 

"extended" 

Color values in the extended dynamic range of the screen are unchanged, and all
other color values are projected to the extended dynamic range of the screen. 

Note: 
This projection is often accomplished by clamping color values in the color space of
the screen to the interval of values that the screen is capable of displaying,
which may include values greater than 1 . 

For example, suppose that the value (2.5, -0.15, -0.15) is written to an
'srgb' canvas.

If this is presented to an sRGB screen that is capable of displaying values
in the [0, 4] interval in sRGB space, then this will be converted to sRGB
(which is a no-op, because the canvas is sRGB), then projected into the display’s space.
If using component-wise clamping, this results in the sRGB value (2.5, 0.0, 0.0) . 

If this is presented to a Display P3 screen that is capable of displaying
values in the [0, 2] interval in Display P3 space, then this will be
converted to the value (2.3, 0.545, 0.386) in the Display P3 color space,
then projected into the display’s space.
If using component-wise clamping, this results in the Display P3 value (2.0, 0.545, 0.386) . 

21.6. GPUCanvasAlphaMode 

This enum selects how the contents of the canvas will be interpreted when read, when
displayed to the screen or used as an image source 
(in drawImage, toDataURL, etc.) 

Below, src is a value in the canvas texture, and dst is an image that the canvas
is being composited into (e.g. an HTML page rendering, or a 2D canvas). 

"opaque" 

Read RGB as opaque and ignore alpha values.
If the content is not already opaque, the alpha channel is cleared to 1.0
in " get a copy of the image contents of a context ". 

"premultiplied" 

Read RGBA as premultiplied: color values are premultiplied by their alpha value.
100% red at 50% alpha is [0.5, 0, 0, 0.5] . 

If the canvas texture contains out-of-gamut premultiplied RGBA values at the time the
canvas contents are read, the behavior depends on whether the canvas is: 

used as an image source 

Values are preserved, as described in color space conversion . 

displayed to the screen

Compositing results are undefined. 

Note: 
This is true even if color space conversion would produce in-gamut values before
compositing, because the intermediate format for compositing is not specified. 

22. Errors & Debugging 

During the normal course of operation of WebGPU, errors are raised via dispatch error . 

After a device is lost , errors are no longer surfaced, where possible.
After this point, implementations do not need to run validation or error tracking: 

The validity of objects on the device becomes unobservable. 

popErrorScope() and uncapturederror stop reporting errors.
(No errors are generated by the device loss itself.
Instead, the GPUDevice . lost promise resolves to indicate the device is lost.) 

All operations which send a message back to the content timeline will skip their usual steps.
Most will appear to succeed, except for mapAsync() , which produces an error
because it is impossible to provide the correct mapped data after the device has been lost. 

This makes it unobservable whether other types of operations (that don’t send messages back)
actually execute or not. 

22.1. Fatal Errors 

enum GPUDeviceLostReason {
"unknown" ,
"destroyed" ,
};

[ Exposed =( Window , Worker ), SecureContext ]
interface GPUDeviceLostInfo {
readonly attribute GPUDeviceLostReason reason ;
readonly attribute DOMString message ;
};

partial interface GPUDevice {
readonly attribute Promise < GPUDeviceLostInfo > " href="#dom-gpudevice-lost" id="ref-for-dom-gpudevice-lost⑥"> lost ;
};

GPUDevice has the following additional attributes: 

lost , of type Promise< GPUDeviceLostInfo >, readonly 

A slot-backed attribute holding a promise which is created with the device, remains
pending for the lifetime of the device, then resolves when the device is lost. 

Upon initialization, it is set to a new promise . 

22.2. GPUError 

[ Exposed =( Window , Worker ), SecureContext ]
interface GPUError {
readonly attribute DOMString message ;
};

GPUError is the base interface for all errors surfaced from popErrorScope() 
and the uncapturederror event. 

Errors must only be generated for operations that explicitly state the conditions one may
be generated under in their respective algorithms, and the subtype of error that is generated. 

No errors are generated from a device which is lost.
See § 22 Errors & Debugging . 

Note: GPUError may gain new subtypes in future versions of this spec. Applications should handle
this possibility, using only the error’s message when possible, and specializing using
instanceof . Use error.constructor.name when it’s necessary to serialize an error (e.g. into
JSON, for a debug report). 

GPUError has the following immutable properties : 

message , of type DOMString , readonly 

A human-readable, localizable text message providing information about the error that
occurred. 

Note: This message is generally intended for application developers to debug their
applications and capture information for debug reports, not to be surfaced to end-users. 

Note: User agents should not include potentially machine-parsable details in this message,
such as free system memory on "out-of-memory" or other details about the
conditions under which memory was exhausted. 

Note: The message should follow the best practices for language and direction information . This includes making use of any future standards which may emerge
regarding the reporting of string language and direction metadata. 

Editorial note: 
At the time of this writing, no language/direction recommendation is available that provides
compatibility and consistency with legacy APIs, but when there is, adopt it formally.

[ Exposed =( Window , Worker ), SecureContext ]
interface GPUValidationError 
: GPUError {
constructor ( DOMString message );
};

GPUValidationError is a subtype of GPUError which indicates that an operation did not
satisfy all validation requirements. Validation errors are always indicative of an application
error, and is expected to fail the same way across all devices assuming the same
[[features]] and [[limits]] are in use. 

To generate a
validation error for GPUDevice device , run the following steps:

Device timeline steps: 

Let error be a new GPUValidationError with an appropriate error message. 

Dispatch error error to device . 

[ Exposed =( Window , Worker ), SecureContext ]
interface GPUOutOfMemoryError 
: GPUError {
constructor ( DOMString message );
};

GPUOutOfMemoryError is a subtype of GPUError which indicates that there was not enough free
memory to complete the requested operation. The operation may succeed if attempted again with a
lower memory requirement (like using smaller texture dimensions), or if memory used by other
resources is released first. 

To 
generate an out-of-memory error for GPUDevice device , run the following steps:

Device timeline steps: 

Let error be a new GPUOutOfMemoryError with an appropriate error message. 

Dispatch error error to device . 

[ Exposed =( Window , Worker ), SecureContext ]
interface GPUInternalError 
: GPUError {
constructor ( DOMString message );
};

GPUInternalError is a subtype of GPUError which indicates than an operation failed for a
system or implementation-specific reason even when all validation requirements have been satisfied.
For example, the operation may exceed the capabilities of the implementation in a way not easily
captured by the supported limits . The same operation may succeed on other devices or under
difference circumstances. 

To generate an
internal error for GPUDevice device , run the following steps:

Device timeline steps: 

Let error be a new GPUInternalError with an appropriate error message. 

Dispatch error error to device . 

22.3. Error Scopes 

A GPU error scope captures GPUError s that were generated while the
GPU error scope was current. Error scopes are used to isolate errors that occur within a set
of WebGPU calls, typically for debugging purposes or to make an operation more fault tolerant. 

GPU error scope has the following device timeline properties : 

[[errors]] , of type list < GPUError >, initially []

The GPUError s, if any, observed while the GPU error scope was current. 

[[filter]] , of type GPUErrorFilter 

Determines what type of GPUError this GPU error scope observes. 

enum GPUErrorFilter {
"validation" ,
"out-of-memory" ,
"internal" ,
};

partial interface GPUDevice {
undefined pushErrorScope ( GPUErrorFilter filter );
Promise < GPUError ?> popErrorScope ();
};

GPUErrorFilter defines the type of errors that should be caught when calling
pushErrorScope() : 

"validation" 

Indicates that the error scope will catch a GPUValidationError . 

"out-of-memory" 

Indicates that the error scope will catch a GPUOutOfMemoryError . 

"internal" 

Indicates that the error scope will catch a GPUInternalError . 

GPUDevice has the following device timeline properties : 

[[errorScopeStack]] , of type stack < GPU error scope >

A stack of GPU error scopes that have been pushed to the GPUDevice . 

The current error scope for a GPUError error and GPUDevice 
device is determined by issuing the following steps to the device timeline of device :

Device timeline steps: 

If error is an instance of: 

GPUValidationError 

Let type be "validation". 

GPUOutOfMemoryError 

Let type be "out-of-memory". 

GPUInternalError 

Let type be "internal". 

Let scope be the last item of device . [[errorScopeStack]] . 

While scope is not undefined : 

If scope . [[filter]] is type , return scope . 

Set scope to the previous item of
device . [[errorScopeStack]] . 

Return undefined . 

To dispatch an error GPUError 
error on GPUDevice device , run the following device timeline steps:

Device timeline steps:

Note: No errors are generated from a device which is lost.
If this algorithm is called while
device is lost , it will not be observable to the application.
See § 22 Errors & Debugging . 

Let scope be the current error scope for error and device . 

If scope is not undefined : 

Append error to scope . [[errors]] . 

Return. 

Otherwise, issue the following steps to the content timeline : 

Content timeline steps:

If the user agent chooses, queue a global task for GPUDevice device 
with the following steps: 

Fire a GPUUncapturedErrorEvent named " uncapturederror " on
device , with an error of error . 

Note: After dispatching the event, user agents should surface uncaptured errors to
developers, for example as warnings in the browser’s developer console, unless the event’s
defaultPrevented is true. In other words, calling preventDefault() 
on the event should silence the console warning. 

Note: The user agent may choose to throttle or limit the number of GPUUncapturedErrorEvent s
that a GPUDevice can raise to prevent an excessive amount of error handling or logging from
impacting performance. 

pushErrorScope(filter) 

Pushes a new GPU error scope onto the [[errorScopeStack]] for this . 

Called on: GPUDevice this .

Arguments: 

Arguments for the GPUDevice.pushErrorScope(filter) method. 

Parameter
Type
Nullable
Optional
Description

filter 
GPUErrorFilter 
✘ 
✘ 
Which class of errors this error scope observes.

Returns: undefined 

Content timeline steps: 

Issue the subsequent steps on the Device timeline of this . 

Device timeline steps:

Let scope be a new GPU error scope . 

Set scope . [[filter]] to filter . 

Push scope onto this . [[errorScopeStack]] . 

popErrorScope() 

Pops a GPU error scope off the [[errorScopeStack]] for this 
and resolves to any GPUError observed by the error scope, or null if none. 

There is no guarantee of the ordering of promise resolution. 

Called on: GPUDevice this .

Returns: Promise < GPUError ?> 

Content timeline steps: 

Let contentTimeline be the current Content timeline . 

Let promise be a new promise . 

Issue the check steps on the Device timeline of this . 

Return promise . 

Device timeline check steps :

If this is lost : 

Issue the following steps on
contentTimeline : 

Content timeline steps:

Resolve promise with null . 

Return. 

Note: No errors are generated from a device which is lost.
See § 22 Errors & Debugging . 

If any of the following requirements are unmet: 

this . [[errorScopeStack]] . size must be > 0. 

Then issue the following steps on contentTimeline 
and return: 

Content timeline steps:

Reject promise with an OperationError . 

Let scope be the result of popping an item off of
this . [[errorScopeStack]] . 

Let error be any one of the items in scope . [[errors]] ,
or null if there are none. 

For any two errors E1 and E2 in the list, if E2 was caused by E1, E2 should
not be the one selected. 

Note: 
For example, if E1 comes from t = createTexture() , and
E2 comes from t . createView() because t was invalid ,
E1 should be be preferred since it will be easier for a developer to understand
what went wrong.
Since both of these are GPUValidationError s, the only difference will be in
the message field, which is meant only to be read by humans anyway. 

At an unspecified point now or in the future ,
issue the subsequent steps on contentTimeline . 

Note: 
By allowing popErrorScope() calls to resolve in any order, with
any of the errors observed by the scope, this spec allows validation to complete
out of order, as long as any state observations are made at the appropriate
point in adherence to this spec. For example, this allows implementations to
perform shader compilation, which depends only on non-stateful inputs, to be
completed on a background thread in parallel with other device-timeline work,
and report any resulting errors later. 

Content timeline steps:

Resolve promise with error . 

Using error scopes to capture validation errors from a GPUDevice operation that may fail:

gpuDevice . pushErrorScope ( 'validation' ); 

let sampler = gpuDevice . createSampler ({ 
maxAnisotropy : 0 , // Invalid, maxAnisotropy must be at least 1. 
}); 

gpuDevice . popErrorScope (). then (( error ) => { 
if ( error ) { 
// There was an error creating the sampler, so discard it. 
sampler = null ; 
console . error ( `An error occured while creating sampler: ${ error . message } ` ); 
} 
}); 

NOTE: 

Error scopes can encompass as many commands as needed. The number of commands an error scope covers
will generally be correlated to what sort of action the application intends to take in response to
an error occuring.

For example: An error scope that only contains the creation of a single resource, such as a texture
or buffer, can be used to detect failures such as out of memory conditions, in which case the
application may try freeing some resources and trying the allocation again. 

Error scopes do not identify which command failed, however. So, for instance, wrapping all the
commands executed while loading a model in a single error scope will not offer enough granularity to
determine if the issue was due to memory constraints. As a result freeing resources would usually
not be a productive response to a failure of that scope. A more appropriate response would be to
allow the application to fall back to a different model or produce a warning that the model could
not be loaded. If responding to memory constraints is desired, the operations allocating memory can
always be wrapped in a smaller nested error scope. 

22.4. Telemetry 

When a GPUError is generated that is not observed by any GPU error scope , the user agent may fire an event named uncapturederror at a GPUDevice using GPUUncapturedErrorEvent . 

Note: uncapturederror events are intended to be used for telemetry and reporting
unexpected errors. They won’t necessarily be dispatched for all uncaptured errors (for example, there may be a limit on the number of errors surfaced), so they should not be used for handling known error cases that may occur during
normal operation of an application. Prefer using pushErrorScope() and
popErrorScope() in those cases. 

[ Exposed =( Window , Worker ), SecureContext ]
interface GPUUncapturedErrorEvent : Event {
constructor (
DOMString type ,
GPUUncapturedErrorEventInit gpuUncapturedErrorEventInitDict 
);
[ SameObject ] readonly attribute GPUError error ;
};

dictionary GPUUncapturedErrorEventInit : EventInit {
required GPUError error ;
};

GPUUncapturedErrorEvent has the following attributes: 

error , of type GPUError , readonly 

A slot-backed attribute holding an object representing the error that was uncaptured.
This has the same type as errors returned by popErrorScope() . 

partial interface GPUDevice {
attribute EventHandler onuncapturederror ;
};

GPUDevice has the following content timeline properties : 

onuncapturederror , of type EventHandler 

An event handler IDL attribute for the uncapturederror event type. 

Listening for uncaptured errors from a GPUDevice :

gpuDevice . addEventListener ( 'uncapturederror' , ( event ) => { 
// Re-surface the error, because adding an event listener may silence console logs. 
console . error ( 'A WebGPU error was not captured:' , event . error ); 

myEngineDebugReport . uncapturedErrors . push ({ 
type : event . error . constructor . name , 
message : event . error . message , 
}); 
}); 

23. Detailed Operations 

This section describes the details of various GPU operations. 

23.1. Computing 

Computing operations provide direct access to GPU’s programmable hardware.
Compute shaders do not have shader stage inputs or outputs; their results are
side effects from writing data into storage bindings bound either as
GPUBufferBindingLayout with GPUBufferBindingType "storage" 
or as GPUStorageTextureBindingLayout .
These operations are encoded within GPUComputePassEncoder as: 

dispatchWorkgroups() 

dispatchWorkgroupsIndirect() 

The main compute algorithm: 

compute ( descriptor , dispatchCall )

Arguments: 

descriptor : Description of the current GPUComputePipeline . 

dispatchCall : The dispatch call parameters. May come from function arguments or an INDIRECT buffer. 

Let computeInvocations be an empty list . 

Let computeStage be descriptor . compute . 

Let workgroupSize be the computed workgroup size for computeStage . entryPoint after
applying computeStage . constants to computeStage . module . 

For workgroupX in range [0, dispatchCall . workgroupCountX ] : 

For workgroupY in range [0, dispatchCall . workgroupCountY ] : 

For workgroupZ in range [0, dispatchCall . workgroupCountZ ] : 

For localX in range [0, workgroupSize . x ] : 

For localY in range [0, workgroupSize . y ] : 

For localZ in range [0, workgroupSize . y ] : 

Let invocation be { computeStage , workgroupX , workgroupY , workgroupZ , localX , localY , localZ } 

Append invocation to computeInvocations . 

For every invocation in computeInvocations , in any order the device chooses, including in parallel: 

Set the shader builtins : 

Set the num_workgroups builtin, if any, to (

dispatchCall . workgroupCountX ,

dispatchCall . workgroupCountY ,

dispatchCall . workgroupCountZ 

) 

Set the workgroup_id builtin, if any, to (

invocation . workgroupX ,

invocation . workgroupY ,

invocation . workgroupZ 

) 

Set the local_invocation_id builtin, if any, to (

invocation . localX ,

invocation . localY ,

invocation . localZ 

) 

Set the global_invocation_id builtin, if any, to (

invocation . workgroupX * workgroupSize . x + invocation . localX ,

invocation . workgroupY * workgroupSize . y + invocation . localY ,

invocation . workgroupZ * workgroupSize . z + invocation . localZ 

) . 

Set the local_invocation_index builtin, if any, to 
invocation . localX + ( invocation . localY * workgroupSize . x ) +
( invocation . localZ * workgroupSize . x * workgroupSize . y )

Invoke the compute shader entry point described by invocation . computeStage . 

Note: Shader invocations have no guaranteed order, and will generally run in parallel according to device
capabilities. Developers should not assume that any given invocation or workgroup will complete before any
other one is started. Some devices may appear to execute in a consistent order, but this behavior should not
be relied on as it will not perform identically across all devices. Shaders that require synchronization
across invocations must use Synchronization Built-in Functions to coordinate execution. 

The device may become lost if
shader execution does not end 
in a reasonable amount of time, as determined by the user agent. 

23.2. Rendering 

Rendering is done by a set of GPU operations that are executed within GPURenderPassEncoder ,
and result in modifications of the texture data, viewed by the render pass attachments.
These operations are encoded with: 

draw() 

drawIndexed() , 

drawIndirect() 

drawIndexedIndirect() . 

Note: rendering is the traditional use of GPUs, and is supported by multiple fixed-function
blocks in hardware. 

The main rendering algorithm: 

render (pipeline, drawCall, state)

Arguments: 

pipeline : The current GPURenderPipeline . 

drawCall : The draw call parameters. May come from function arguments or an INDIRECT buffer. 

state : RenderState of the GPURenderCommandsMixin where the draw call is issued. 

Let descriptor be pipeline . [[descriptor]] . 

Resolve indices . See § 23.2.1 Index Resolution . 

Let vertexList be the result of resolve indices ( drawCall , state ). 

Process vertices . See § 23.2.2 Vertex Processing . 

Execute process vertices ( vertexList , drawCall , descriptor . vertex , state ). 

Assemble primitives . See § 23.2.3 Primitive Assembly . 

Execute assemble primitives ( vertexList , drawCall , descriptor . primitive ). 

Clip primitives . See § 23.2.4 Primitive Clipping . 

Let primitiveList be the result of this stage. 

Rasterize . See § 23.2.5 Rasterization . 

Let rasterizationList be the result of rasterize ( primitiveList , state ). 

Process fragments . See § 23.2.6 Fragment Processing . 

Gather a list of fragments , resulting from executing
process fragment ( rasterPoint , descriptor , state )
for each rasterPoint in rasterizationList . 

Write pixels . See § 23.2.7 Output Merging . 

For each non-null fragment of fragments : 

Execute process depth stencil ( fragment , pipeline , state ). 

Execute process color attachments ( fragment , pipeline , state ). 

23.2.1. Index Resolution 

At the first stage of rendering, the pipeline builds
a list of vertices to process for each instance. 

resolve indices (drawCall, state)

Arguments: 

drawCall : The draw call parameters. May come from function arguments or an INDIRECT buffer. 

state : The snapshot of the GPURenderCommandsMixin state at the time of the draw call. 

Returns: list of integer indices. 

Let vertexIndexList be an empty list of indices. 

If drawCall is an indexed draw call: 

Initialize the vertexIndexList with drawCall .indexCount integers. 

For i in range 0 .. drawCall .indexCount (non-inclusive): 

Let relativeVertexIndex be fetch index ( i + drawCall . firstIndex ,
state . [[index_buffer]] ). 

If relativeVertexIndex has the special value "out of bounds" ,
return the empty list. 

Note: Implementations may choose to display a warning when this occurs,
especially when it is easy to detect (like in non-indirect indexed draw calls). 

Append drawCall . baseVertex + relativeVertexIndex to the vertexIndexList . 

Otherwise: 

Initialize the vertexIndexList with drawCall .vertexCount integers. 

Set each vertexIndexList item i to the value drawCall .firstVertex + i . 

Return vertexIndexList . 

Note: in the case of indirect draw calls, the indexCount , vertexCount ,
and other properties of drawCall are read from the indirect buffer
instead of the draw command itself. 

fetch index (i, buffer, offset, format)

Arguments: 

i : Index of a vertex index to fetch. 

state : The snapshot of the GPURenderCommandsMixin state at the time of the draw call. 

Returns: unsigned integer or "out of bounds" 

Let indexSize be defined by the state . [[index_format]] : 

"uint16" 

2 

"uint32" 

4 

If state . [[index_buffer_offset]] +
|i + 1| × indexSize > state . [[index_buffer_size]] ,
return the special value "out of bounds" . 

Interpret the data in state . [[index_buffer]] , starting at offset
state . [[index_buffer_offset]] + i × indexSize ,
of size indexSize bytes, as an unsigned integer and return it. 

23.2.2. Vertex Processing 

Vertex processing stage is a programmable stage of the render pipeline that
processes the vertex attribute data, and produces
clip space positions for § 23.2.4 Primitive Clipping , as well as other data for the
§ 23.2.6 Fragment Processing . 

process vertices (vertexIndexList, drawCall, desc, state)

Arguments: 

vertexIndexList : List of vertex indices to process (mutable, passed by reference). 

drawCall : The draw call parameters. May come from function arguments or an INDIRECT buffer. 

desc : The descriptor of type GPUVertexState . 

state : The snapshot of the GPURenderCommandsMixin state at the time of the draw call. 

Each vertex vertexIndex in the vertexIndexList ,
in each instance of index rawInstanceIndex , is processed independently.
The rawInstanceIndex is in range from 0 to drawCall .instanceCount - 1, inclusive.
This processing happens in parallel, and any side effects, such as
writes into GPUBufferBindingType "storage" bindings,
may happen in any order. 

Let instanceIndex be rawInstanceIndex + drawCall .firstInstance. 

For each non- null vertexBufferLayout in the list of desc . buffers : 

Let i be the index of the buffer layout in this list. 

Let vertexBuffer , vertexBufferOffset , and vertexBufferBindingSize be the
buffer, offset, and size at slot i of state . [[vertex_buffers]] . 

Let vertexElementIndex be dependent on vertexBufferLayout . stepMode : 

"vertex" 

vertexIndex 

"instance" 

instanceIndex 

Let drawCallOutOfBounds be false . 

For each attributeDesc in vertexBufferLayout . attributes : 

Let attributeOffset be vertexBufferOffset +
vertexElementIndex * vertexBufferLayout . arrayStride +
attributeDesc . offset . 

If attributeOffset + byteSize ( attributeDesc . format ) >
vertexBufferOffset + vertexBufferBindingSize : 

Set drawCallOutOfBounds to true . 

Optionally ( implementation-defined ) ,
empty vertexIndexList and return, cancelling the draw call. 

Note: This allows implementations to detect out-of-bounds values in the index buffer
before issuing a draw call, instead of using invalid memory reference behavior. 

For each attributeDesc in vertexBufferLayout . attributes : 

If drawCallOutOfBounds is true : 

Load the attribute data according to WGSL’s invalid memory reference 
behavior, from vertexBuffer . 

Note: Invalid memory reference allows several behaviors, including actually
loading the "correct" result for an attribute that is in-bounds, even when
the draw-call-wide drawCallOutOfBounds is true . 

Otherwise: 

Let attributeOffset be vertexBufferOffset +
vertexElementIndex * vertexBufferLayout . arrayStride +
attributeDesc . offset . 

Load the attribute data of format attributeDesc . format 
from vertexBuffer starting at offset attributeOffset .
The components are loaded in the order x , y , z , w from buffer memory. 

Convert the data into a shader-visible format, according to channel formats rules. 

An attribute of type "snorm8x2" and byte values of [0x70, 0xD0] 
will be converted to vec2<f32>(0.88, -0.38) in WGSL.

Adjust the data size to the shader type: 

if both are scalar, or both are vectors of the same dimensionality, no adjustment is needed. 

if data is vector but the shader type is scalar, then only the first component is extracted. 

if both are vectors, and data has a higher dimension, the extra components are dropped. 

An attribute of type "float32x3" and value vec3<f32>(1.0, 2.0, 3.0) 
will exposed to the shader as vec2<f32>(1.0, 2.0) if a 2-component vector is expected.

if the shader type is a vector of higher dimensionality, or the data is a scalar,
then the missing components are filled from vec4<*>(0, 0, 0, 1) value. 

An attribute of type "sint32" and value 5 will be exposed
to the shader as vec4<i32>(5, 0, 0, 1) if a 4-component vector is expected.

Bind the data to vertex shader input
location attributeDesc . shaderLocation . 

For each GPUBindGroup group at index in state . [[bind_groups]] : 

For each resource GPUBindingResource in the bind group: 

Let entry be the corresponding GPUBindGroupLayoutEntry for this resource. 

If entry . visibility includes VERTEX : 

Bind the resource to the shader under group index and binding GPUBindGroupLayoutEntry.binding . 

Set the shader builtins : 

Set the vertex_index builtin, if any, to vertexIndex . 

Set the instance_index builtin, if any, to instanceIndex . 

Invoke the vertex shader entry point described by desc . 

Note: The target platform caches the results of vertex shader invocations.
There is no guarantee that any vertexIndex that repeats more than once will
result in multiple invocations. Similarly, there is no guarantee that a single vertexIndex 
will only be processed once. 

The device may become lost if
shader execution does not end 
in a reasonable amount of time, as determined by the user agent. 

23.2.3. Primitive Assembly 

Primitives are assembled by a fixed-function stage of GPUs. 

assemble primitives (vertexIndexList, drawCall, desc)

Arguments: 

vertexIndexList : List of vertex indices to process. 

drawCall : The draw call parameters. May come from function arguments or an INDIRECT buffer. 

desc : The descriptor of type GPUPrimitiveState . 

For each instance, the primitives get assembled from the vertices that have been
processed by the shaders, based on the vertexIndexList . 

First, if the primitive topology is a strip, (which means that
desc . stripIndexFormat is not undefined)
and the drawCall is indexed, the vertexIndexList is split into
sub-lists using the maximum value of desc . stripIndexFormat 
as a separator. 

Example: a vertexIndexList with values [1, 2, 65535, 4, 5, 6] of type "uint16" 
will be split in sub-lists [1, 2] and [4, 5, 6] . 

For each of the sub-lists vl , primitive generation is done according to the
desc . topology : 

"line-list" 

Line primitives are composed from ( vl .0, vl .1),
then ( vl .2, vl .3), then ( vl .4 to vl .5), etc.
Each subsequent primitive takes 2 vertices. 

"line-strip" 

Line primitives are composed from ( vl .0, vl .1),
then ( vl .1, vl .2), then ( vl .2, vl .3), etc.
Each subsequent primitive takes 1 vertex. 

"triangle-list" 

Triangle primitives are composed from ( vl .0, vl .1, vl .2),
then ( vl .3, vl .4, vl .5), then ( vl .6, vl .7, vl .8), etc.
Each subsequent primitive takes 3 vertices. 

"triangle-strip" 

Triangle primitives are composed from ( vl .0, vl .1, vl .2),
then ( vl .2, vl .1, vl .3), then ( vl .2, vl .3, vl .4),
then ( vl .4, vl .3, vl .5), etc.
Each subsequent primitive takes 1 vertices. 

Any incomplete primitives are dropped. 

23.2.4. Primitive Clipping 

Vertex shaders have to produce a built-in position (of type vec4<f32> ),
which denotes the clip position of a vertex in clip space coordinates . 

Primitives are clipped to the clip volume , which, for any clip position p 
inside a primitive, is defined by the following inequalities: 

− p .w ≤ p .x ≤ p .w 

− p .w ≤ p .y ≤ p .w 

0 ≤ p .z ≤ p .w ( depth clipping ) 

When the "clip-distances" feature is enabled, this clip volume can
be further restricted by user-defined half-spaces by declaring clip_distances in the
output of vertex stage. Each value in the clip_distances array will be linearly
interpolated across the primitive, and the portion of the primitive with interpolated distances less
than 0 will be clipped. 

If descriptor . primitive . unclippedDepth is true ,
depth clipping is not applied: the clip volume is not bounded in the z dimension. 

A primitive passes through this stage unchanged if every one of its edges
lie entirely inside the clip volume .
If the edges of a primitives intersect the boundary of the clip volume ,
the intersecting edges are reconnected by new edges that lie along the boundary of the clip volume .
For triangular primitives ( descriptor . primitive . topology is
"triangle-list" or "triangle-strip" ), this reconnection
may result in introduction of new vertices into the polygon, internally. 

If a primitive intersects an edge of the clip volume ’s boundary,
the clipped polygon must include a point on this boundary edge. 

If the vertex shader outputs other floating-point values (scalars and vectors), qualified with
"perspective" interpolation, they also get clipped.
The output values associated with a vertex that lies within the clip volume are unaffected by clipping.
If a primitive is clipped, however, the output values assigned to vertices produced by clipping are clipped. 

Considering an edge between vertices a and b that got clipped, resulting in the vertex c ,
let’s define t to be the ratio between the edge vertices:
c .p = t × a .p + (1 − t ) × b .p,
where x .p is the output clip position of a vertex x . 

For each vertex output value "v" with a corresponding fragment input,
a .v and b .v would be the outputs for a and b vertices respectively.
The clipped shader output c .v is produced based on the interpolation qualifier: 

flat 

Flat interpolation is unaffected, and is based on the provoking vertex ,
which is determined by the interpolation sampling mode declared in the shader. The
output value is the same for the whole primitive, and matches the vertex output of the
provoking vertex . 

linear 

The interpolation ratio gets adjusted against the perspective coordinates of the
clip position s, so that the result of interpolation is linear in screen space. 

perspective 

The value is linearly interpolated in clip space, producing perspective-correct values. 

The result of primitive clipping is a new set of primitives, which are contained
within the clip volume . 

23.2.5. Rasterization 

Rasterization is the hardware processing stage that maps the generated primitives
to the 2-dimensional rendering area of the framebuffer -
the set of render attachments in the current GPURenderPassEncoder .
This rendering area is split into an even grid of pixels. 

The framebuffer coordinates start from the top-left corner of the render targets.
Each unit corresponds exactly to one pixel. See § 3.3 Coordinate Systems for more information. 

Rasterization determines the set of pixels affected by a primitive. In case of multi-sampling,
each pixel is further split into
descriptor . multisample . count samples.
The standard sample patterns are as follows,
with positions in framebuffer coordinates relative to the top-left corner of the pixel,
such that the pixel ranges from (0, 0) to (1, 1): 

multisample . count 
Sample positions

1

Sample 0: (0.5, 0.5)

4

Sample 0: (0.375, 0.125)

Sample 1: (0.875, 0.375)

Sample 2: (0.125, 0.625)

Sample 3: (0.625, 0.875)

Implementations must use the standard sample pattern for the given
multisample . count when performing rasterization. 

Let’s define a FragmentDestination to contain: 

position 

the 2D pixel position using framebuffer coordinates 

sampleIndex 

an integer in case § 23.2.10 Per-Sample Shading is active,
or null otherwise 

We’ll also use a notion of normalized device coordinates , or NDC.
In this coordinate system, the viewport bounds range in X and Y from -1 to 1, and in Z from 0 to 1. 

Rasterization produces a list of RasterizationPoint s, each containing the following data: 

destination 

refers to FragmentDestination 

coverageMask 

refers to multisample coverage mask (see § 23.2.11 Sample Masking ) 

frontFacing 

is true if it’s a point on the front face of a primitive 

perspectiveDivisor 

refers to interpolated 1.0 ÷ W across the primitive 

depth 

refers to the depth in viewport coordinates ,
i.e. between the [[viewport]] minDepth and maxDepth . 

primitiveVertices 

refers to the list of vertex outputs forming the primitive 

barycentricCoordinates 

refers to § 23.2.5.3 Barycentric coordinates 

rasterize (primitiveList, state)

Arguments: 

primitiveList : List of primitives to rasterize. 

state : The active RenderState . 

Returns: list of RasterizationPoint . 

Each primitive in primitiveList is processed independently.
However, the order of primitives affects later stages, such as depth/stencil operations and pixel writes. 

First, the clipped vertices are transformed into NDC - normalized device coordinates.
Given the output position p , the NDC position and perspective divisor are: 

ndc( p ) = vector( p .x ÷ p .w, p .y ÷ p .w, p .z ÷ p .w) 

divisor( p ) = 1.0 ÷ p .w 

Let vp be state . [[viewport]] .
Map the NDC position n into viewport coordinates : 

Compute framebuffer coordinates from the render target offset and size: 

framebufferCoords( n ) = vector( vp . x + 0.5 × ( n .x + 1) × vp . width , vp . y + 0.5 × (− n .y + 1) × vp . height ) 

Compute depth by linearly mapping [0,1] to the viewport depth range: 

depth( n ) = vp . minDepth + n . z × ( vp . maxDepth - vp . minDepth ) 

Let rasterizationPoints be the list of points, each having its attributes ( divisor(p) ,
framebufferCoords(n) , depth(n) , etc.) interpolated according to its position on the
primitive, using the same interpolation as § 23.2.4 Primitive Clipping . If the attribute is
user-defined (not a built-in output value ) then the interpolation type specified by
the @interpolate WGSL attribute is used. 

Proceed with a specific rasterization algorithm,
depending on primitive . topology : 

"point-list" 

The point, if not filtered by § 23.2.4 Primitive Clipping , goes into § 23.2.5.1 Point Rasterization . 

"line-list" or "line-strip" 

The line cut by § 23.2.4 Primitive Clipping goes into § 23.2.5.2 Line Rasterization . 

"triangle-list" or "triangle-strip" 

The polygon produced in § 23.2.4 Primitive Clipping goes into § 23.2.5.4 Polygon Rasterization . 

Remove all the points rp from rasterizationPoints that have
rp . destination . position 
outside of state . [[scissorRect]] . 

Return rasterizationPoints . 

23.2.5.1. Point Rasterization 

A single FragmentDestination is selected within the pixel containing the
framebuffer coordinates of the point. 

The coverage mask depends on multi-sampling mode: 

sample-frequency

coverageMask = 1 ≪ sampleIndex 

pixel-frequency multi-sampling

coverageMask = 1 ≪ descriptor . multisample . count − 1 

no multi-sampling

coverageMask = 1 

23.2.5.2. Line Rasterization 

The exact algorithm used for line rasterization is not defined, and may differ between
implementations. For example, the line may be drawn using § 23.2.5.4 Polygon Rasterization of a 1px-width
rectangle around the line segment, or using Bresenham’s line algorithm to select the
FragmentDestination s. 

Note: See Basic Line Segment Rasterization and
Bresenham Line Segment Rasterization in the Vulkan 1.3 
spec for more details of how line these line rasterization algorithms may be implemented. 

23.2.5.3. Barycentric coordinates 

Barycentric coordinates is a list of n numbers b i ,
defined for a point p inside a convex polygon with n vertices v i in framebuffer space.
Each b i is in range 0 to 1, inclusive, and represents the proximity to vertex v i .
Their sum is always constant: 

∑ ( b i ) = 1 

These coordinates uniquely specify any point p within the polygon (or on its boundary) as: 

p = ∑ ( b i × p i ) 

For a polygon with 3 vertices - a triangle,
barycentric coordinates of any point p can be computed as follows: 

A polygon = A( v 1 , v 2 , v 3 )
b 1 = A( p , b 2 , b 3 ) ÷ A polygon 
b 2 = A( b 1 , p , b 3 ) ÷ A polygon 
b 3 = A( b 1 , b 2 , p ) ÷ A polygon 

Where A(list of points) is the area of the polygon with the given set of vertices. 

For polygons with more than 3 vertices, the exact algorithm is implementation-dependent.
One of the possible implementations is to triangulate the polygon and compute the barycentrics
of a point based on the triangle it falls into. 

23.2.5.4. Polygon Rasterization 

A polygon is front-facing if it’s oriented towards the projection.
Otherwise, the polygon is back-facing . 

rasterize polygon ()

Arguments: 

Returns: list of RasterizationPoint . 

Let rasterizationPoints be an empty list. 

Let v ( i ) be the framebuffer coordinates for the clipped vertex number i (starting with 1)
in a rasterized polygon of n vertices. 

Note: this section uses the term "polygon" instead of a "triangle",
since § 23.2.4 Primitive Clipping stage may have introduced additional vertices.
This is non-observable by the application. 

Determine if the polygon is front-facing,
which depends on the sign of the area occupied by the polygon in framebuffer coordinates: 

area = 0.5 × (( v 1 .x × v n .y − v n .x × v 1 .y) + ∑ ( v i +1 .x × v i .y − v i .x × v i +1 .y)) 

The sign of area is interpreted based on the primitive . frontFace : 

"ccw" 

area > 0 is considered front-facing , otherwise back-facing 

"cw" 

area < 0 is considered front-facing , otherwise back-facing 

Cull based on primitive . cullMode : 

"none" 

All polygons pass this test. 

"front" 

The front-facing polygons are discarded,
and do not process in later stages of the render pipeline. 

"back" 

The back-facing polygons are discarded. 

Determine a set of fragments inside the polygon in framebuffer space -
these are locations scheduled for the per-fragment operations.
This operation is known as "point sampling".
The logic is based on descriptor . multisample : 

disabled

Fragment s are associated with pixel centers. That is, all the points with coordinates C , where
fract( C ) = vector2(0.5, 0.5) in the framebuffer space, enclosed into the polygon, are included.
If a pixel center is on the edge of the polygon, whether or not it’s included is not defined. 

Note: this becomes a subject of precision for the rasterizer. 

enabled

Each pixel is associated with descriptor . multisample . count 
locations, which are implementation-defined .
The locations are ordered, and the list is the same for each pixel of the framebuffer .
Each location corresponds to one fragment in the multisampled framebuffer . 

The rasterizer builds a mask of locations being hit inside each pixel and provides is as "sample-mask"
built-in to the fragment shader. 

For each produced fragment of type FragmentDestination : 

Let rp be a new RasterizationPoint object 

Compute the list b as § 23.2.5.3 Barycentric coordinates of that fragment.
Set rp . barycentricCoordinates to b . 

Let d i be the depth value of v i . 

Set rp . depth to ∑ ( b i × d i ) 

Append rp to rasterizationPoints . 

Return rasterizationPoints . 

23.2.6. Fragment Processing 

The fragment processing stage is a programmable stage of the render pipeline that
computes the fragment data (often a color) to be written into render targets. 

This stage produces a Fragment for each RasterizationPoint : 

destination refers to FragmentDestination . 

frontFacing is true if it’s a fragment on the front face of a primitive. 

coverageMask refers to multisample coverage mask (see § 23.2.11 Sample Masking ). 

depth refers to the depth in viewport coordinates ,
i.e. between the [[viewport]] minDepth and maxDepth . 

colors refers to the list of color values,
one for each target in colorAttachments . 

depthPassed 
is true if the fragment passed the depthCompare operation. 

stencilPassed 
is true if the fragment passed the stencil compare operation. 

process fragment (rp, descriptor, state)

Arguments: 

rp : The RasterizationPoint , produced by § 23.2.5 Rasterization . 

descriptor : The descriptor of type GPURenderPipelineDescriptor . 

state : The active RenderState . 

Returns: Fragment or null . 

Let fragmentDesc be descriptor . fragment . 

Let depthStencilDesc be descriptor . depthStencil . 

Let fragment be a new Fragment object. 

Set fragment . destination to rp . destination . 

Set fragment . frontFacing to rp . frontFacing . 

Set fragment . coverageMask to rp . coverageMask . 

Set fragment . depth to rp . depth . 

If frag_depth builtin is not produced by the shader: 

Set fragment . depthPassed to the result of compare fragment ( fragment . destination ,
fragment . depth , " depth ", state . [[depthStencilAttachment]] ,
depthStencilDesc ?. depthCompare ). 

Set stencilState to depthStencilDesc ?. stencilFront if
rp . frontFacing is true and depthStencilDesc ?. stencilBack 
otherwise. 

Set fragment . stencilPassed to the result of compare fragment ( fragment . destination ,
state . [[stencilReference]] , " stencil ", state . [[depthStencilAttachment]] ,
stencilState ?. compare ). 

If fragmentDesc is not null : 

If fragment . depthPassed is false , the frag_depth builtin is not produced by the
shader entry point, and the shader entry point does not write to any storage bindings,
the following steps may be skipped. 

Set the shader input builtins . For each non-composite argument of the entry point,
annotated as a builtin , set its value based on the annotation: 

position 

vec4<f32> ( rp . destination . position , rp . depth , rp . perspectiveDivisor ) 

front_facing 

rp . frontFacing 

sample_index 

rp . destination . sampleIndex 

sample_mask 

rp . coverageMask 

For each user-specified shader stage input of the fragment stage: 

Let value be the interpolated fragment input,
based on rp . barycentricCoordinates , rp . primitiveVertices ,
and the interpolation qualifier on the input. 

Set the corresponding fragment shader location input to value . 

Invoke the fragment shader entry point described by fragmentDesc . 

The device may become lost if
shader execution does not end 
in a reasonable amount of time, as determined by the user agent. 

If the fragment issued discard , return null . 

Set fragment . colors to the user-specified shader stage output values from the shader. 

Take the shader output builtins : 

If frag_depth builtin is produced by the shader as value : 

Let vp be state . [[viewport]] . 

Set fragment . depth to clamp( value , vp . minDepth , vp . maxDepth ). 

Set fragment . depthPassed to the result of compare fragment ( fragment . destination ,
fragment . depth , " depth ", state . [[depthStencilAttachment]] ,
depthStencilDesc ?. depthCompare ). 

If sample_mask builtin is produced by the shader as value : 

Set fragment . coverageMask to fragment . coverageMask ∧ value . 

Otherwise we are in § 23.2.8 No Color Output mode, and fragment . colors is empty. 

Return fragment . 

compare fragment (destination, value, aspect, attachment, compareFunc)

Arguments: 

destination : The FragmentDestination . 

value : The value to be compared. 

aspect : The aspect of attachment to sample values from. 

attachment : The attachment to be compared against. 

compareFunc : The GPUCompareFunction to use, or undefined . 

Returns: true if the comparison passes, or false otherwise 

If attachment is undefined or does not have aspect , return true . 

If compareFunc is undefined or "always" , return true . 

Let attachmentValue be the value of aspect of attachment at destination . 

Return true if comparing value with attachmentValue using compareFunc succeeds, and false otherwise. 

Processing of fragments happens in parallel, while any side effects,
such as writes into GPUBufferBindingType "storage" bindings,
may happen in any order. 

23.2.7. Output Merging 

Output merging is a fixed-function stage of the render pipeline that
outputs the fragment color, depth and stencil data to be written into the render pass attachments. 

process depth stencil (fragment, pipeline, state)

Arguments: 

fragment : The Fragment , produced by § 23.2.6 Fragment Processing . 

pipeline : The current GPURenderPipeline . 

state : The active RenderState . 

Let depthStencilDesc be pipeline . [[descriptor]] . depthStencil . 

If pipeline . [[writesDepth]] is true and fragment . depthPassed is true : 

Set the value of the depth aspect of state . [[depthStencilAttachment]] at
fragment . destination to fragment . depth . 

If pipeline . [[writesStencil]] is true: 

Set stencilState to depthStencilDesc . stencilFront if
fragment . frontFacing is true and
depthStencilDesc . stencilBack otherwise. 

If fragment . stencilPassed is false : 

Let stencilOp be stencilState . failOp . 

Otherwise, if fragment . depthPassed is false : 

Let stencilOp be stencilState . depthFailOp . 

Otherwise: 

Let stencilOp be stencilState . passOp . 

Update the value of the stencil aspect of state . [[depthStencilAttachment]] at
fragment . destination by performing the operation described by stencilOp . 

The depth input to this stage, if any, is clamped to the current [[viewport]] depth
range (regardless of whether the fragment shader stage writes the frag_depth builtin). 

process color attachments (fragment, pipeline, state)

Arguments: 

fragment : The Fragment , produced by § 23.2.6 Fragment Processing . 

pipeline : The current GPURenderPipeline . 

state : The active RenderState . 

If fragment . depthPassed is false or fragment . stencilPassed is false , return. 

Let targets be pipeline . [[descriptor]] . fragment . targets . 

For each attachment of state . [[colorAttachments]] : 

Let color be the value from fragment . colors that corresponds with attachment . 

Let targetDesc be the targets entry that corresponds with attachment . 

If targetDesc . blend is provided : 

Let colorBlend be targetDesc . blend . color . 

Let alphaBlend be targetDesc . blend . alpha . 

Set the RGB components of color to the value computed by performing the operation described by
colorBlend . operation with the values described by
colorBlend . srcFactor and colorBlend . dstFactor . 

Set the alpha component of color to the value computed by performing the operation described by
alphaBlend . operation with the values described by
alphaBlend . srcFactor and alphaBlend . dstFactor . 

Set the value of attachment at fragment . destination to color . 

23.2.8. No Color Output 

In no-color-output mode, pipeline does not produce any color attachment outputs. 

The pipeline still performs rasterization and produces depth values
based on the vertex position output. The depth testing and stencil operations can still be used. 

23.2.9. Alpha to Coverage 

In alpha-to-coverage mode, an additional alpha-to-coverage mask 
of MSAA samples is generated based on the alpha component of the
fragment shader output value at @location(0) . 

The algorithm of producing the extra mask is platform-dependent and can vary for different pixels.
It guarantees that: 

if alpha ≤ 0.0, the result is 0x0 

if alpha ≥ 1.0, the result is 0xFFFFFFFF 

intermediate alpha values should result in a proportionate number of bits set to 1 in the mask.
Not all platforms guarantee that the number of bits set to 1 in the mask monotonically
increases as alpha increases for a given pixel. 

23.2.10. Per-Sample Shading

When rendering into multisampled render attachments, fragment shaders can be run once per-pixel or once per-sample.
Fragment shaders must run once per-sample if either the sample_index builtin or sample interpolation sampling 
is used and contributes to the shader output. Otherwise fragment shaders may run once per-pixel with the result
broadcast out to each of the samples included in the final sample mask . 

When using per-sample shading, the color output for sample N is produced by the fragment shader execution
with sample_index == N for the current pixel. 

23.2.11. Sample Masking 

The final sample mask for a pixel is computed as:
rasterization mask & mask & shader-output mask . 

Only the lower count bits of the mask are considered. 

If the least-significant bit at position N of the final sample mask has value of "0",
the sample color outputs (corresponding to sample N ) to all attachments of the fragment shader are discarded.
Also, no depth test or stencil operations are executed on the relevant samples of the depth-stencil attachment. 

The rasterization mask is produced by the rasterization stage,
based on the shape of the rasterized polygon. The samples included in the shape get the relevant
bits 1 in the mask. 

The shader-output mask takes the output value of "sample_mask" builtin in the fragment shader.
If the builtin is not output from the fragment shader, and alphaToCoverageEnabled 
is enabled, the shader-output mask becomes the alpha-to-coverage mask . Otherwise, it defaults to 0xFFFFFFFF. 

24. Type Definitions 

typedef [ EnforceRange ] unsigned long GPUBufferDynamicOffset ;
typedef [ EnforceRange ] unsigned long GPUStencilValue ;
typedef [ EnforceRange ] unsigned long GPUSampleMask ;
typedef [ EnforceRange ] long GPUDepthBias ;

typedef [ EnforceRange ] unsigned long long GPUSize64 ;
typedef [ EnforceRange ] unsigned long GPUIntegerCoordinate ;
typedef [ EnforceRange ] unsigned long GPUIndex32 ;
typedef [ EnforceRange ] unsigned long GPUSize32 ;
typedef [ EnforceRange ] long GPUSignedOffset32 ;

typedef unsigned long long GPUSize64Out ;
typedef unsigned long GPUIntegerCoordinateOut ;
typedef unsigned long GPUSize32Out ;

typedef unsigned long GPUFlagsConstant ;

24.1. Colors & Vectors 

dictionary GPUColorDict {
required double r ;
required double g ;
required double b ;
required double a ;
};
typedef ( sequence < double > or GPUColorDict ) GPUColor ;

Note: double is large enough to precisely hold 32-bit signed/unsigned
integers and single-precision floats. 

r , of type double 

The red channel value. 

g , of type double 

The green channel value. 

b , of type double 

The blue channel value. 

a , of type double 

The alpha channel value. 

For a given GPUColor value color , depending on its type, the syntax:

color . r refers to
either GPUColorDict . r 
or the first item of the sequence ( asserting there is such an item). 

color . g refers to
either GPUColorDict . g 
or the second item of the sequence ( asserting there is such an item). 

color . b refers to
either GPUColorDict . b 
or the third item of the sequence ( asserting there is such an item). 

color . a refers to
either GPUColorDict . a 
or the fourth item of the sequence ( asserting there is such an item). 

validate GPUColor shape (color)

Arguments: 

color : The GPUColor to validate. 

Returns: undefined 

Content timeline steps: 

Throw a TypeError if color is a sequence and color . size ≠ 4. 

dictionary GPUOrigin2DDict {
GPUIntegerCoordinate x = 0;
GPUIntegerCoordinate y = 0;
};
typedef ( sequence < GPUIntegerCoordinate > or GPUOrigin2DDict ) GPUOrigin2D ;

For a given GPUOrigin2D value origin , depending on its type, the syntax:

origin . x refers to
either GPUOrigin2DDict . x 
or the first item of the sequence (0 if not present). 

origin . y refers to
either GPUOrigin2DDict . y 
or the second item of the sequence (0 if not present). 

validate GPUOrigin2D shape (origin)

Arguments: 

origin : The GPUOrigin2D to validate. 

Returns: undefined 

Content timeline steps: 

Throw a TypeError if origin is a sequence and origin . size > 2. 

dictionary GPUOrigin3DDict {
GPUIntegerCoordinate x = 0;
GPUIntegerCoordinate y = 0;
GPUIntegerCoordinate z = 0;
};
typedef ( sequence < GPUIntegerCoordinate > or GPUOrigin3DDict ) GPUOrigin3D ;

For a given GPUOrigin3D value origin , depending on its type, the syntax:

origin . x refers to
either GPUOrigin3DDict . x 
or the first item of the sequence (0 if not present). 

origin . y refers to
either GPUOrigin3DDict . y 
or the second item of the sequence (0 if not present). 

origin . z refers to
either GPUOrigin3DDict . z 
or the third item of the sequence (0 if not present). 

validate GPUOrigin3D shape (origin)

Arguments: 

origin : The GPUOrigin3D to validate. 

Returns: undefined 

Content timeline steps: 

Throw a TypeError if origin is a sequence and origin . size > 3. 

dictionary GPUExtent3DDict {
required GPUIntegerCoordinate width ;
GPUIntegerCoordinate height = 1;
GPUIntegerCoordinate depthOrArrayLayers = 1;
};
typedef ( sequence < GPUIntegerCoordinate > or GPUExtent3DDict ) GPUExtent3D ;

width , of type GPUIntegerCoordinate 

The width of the extent. 

height , of type GPUIntegerCoordinate , defaulting to 1 

The height of the extent. 

depthOrArrayLayers , of type GPUIntegerCoordinate , defaulting to 1 

The depth of the extent or the number of array layers it contains.
If used with a GPUTexture with a GPUTextureDimension of "3d" 
defines the depth of the texture. If used with a GPUTexture with a GPUTextureDimension 
of "2d" defines the number of array layers in the texture. 

For a given GPUExtent3D value extent , depending on its type, the syntax:

extent . width refers to
either GPUExtent3DDict . width 
or the first item of the sequence ( asserting there is such an item). 

extent . height refers to
either GPUExtent3DDict . height 
or the second item of the sequence (1 if not present). 

extent . depthOrArrayLayers refers to
either GPUExtent3DDict . depthOrArrayLayers 
or the third item of the sequence (1 if not present). 

validate GPUExtent3D shape (extent)

Arguments: 

extent : The GPUExtent3D to validate. 

Returns: undefined 

Content timeline steps: 

Throw a TypeError if: 

extent is a sequence, and 

extent . size < 1 or extent . size > 3. 

25. Feature Index 

25.1. "core-features-and-limits" 

Allows all Core WebGPU features and limits to be used. 

This is always available unless featureLevel is set to "compatibility" ,
in which case it may or may not be available (see those definitions for information). 

25.2. "depth-clip-control" 

Allows depth clipping to be disabled. 

This feature adds the following optional API surfaces : 

New GPUPrimitiveState dictionary members: 

unclippedDepth 

25.3. "depth32float-stencil8" 

Allows for explicit creation of textures of format "depth32float-stencil8" . 

This feature adds the following optional API surfaces : 

New GPUTextureFormat enum values: 

"depth32float-stencil8" 

25.4. "texture-compression-bc" 

Allows for explicit creation of textures of BC compressed formats which include the "S3TC",
"RGTC", and "BPTC" formats. Only supports 2D textures. 

Note: Adapters which support "texture-compression-bc" do not
always support "texture-compression-bc-sliced-3d" .
To use "texture-compression-bc-sliced-3d" ,
"texture-compression-bc" must be enabled explicitly as this feature
does not enable the BC formats. 

This feature adds the following optional API surfaces : 

New GPUTextureFormat enum values: 

"bc1-rgba-unorm" 

"bc1-rgba-unorm-srgb" 

"bc2-rgba-unorm" 

"bc2-rgba-unorm-srgb" 

"bc3-rgba-unorm" 

"bc3-rgba-unorm-srgb" 

"bc4-r-unorm" 

"bc4-r-snorm" 

"bc5-rg-unorm" 

"bc5-rg-snorm" 

"bc6h-rgb-ufloat" 

"bc6h-rgb-float" 

"bc7-rgba-unorm" 

"bc7-rgba-unorm-srgb" 

25.5. "texture-compression-bc-sliced-3d" 

Allows the 3d dimension for textures with BC compressed formats . 

Note: Adapters which support "texture-compression-bc" do not
always support "texture-compression-bc-sliced-3d" .
To use "texture-compression-bc-sliced-3d" ,
"texture-compression-bc" must be enabled explicitly as this feature
does not enable the BC formats. 

This feature adds no optional API surfaces . 

25.6. "texture-compression-etc2" 

Allows for explicit creation of textures of ETC2 compressed formats . Only supports 2D textures. 

This feature adds the following optional API surfaces : 

New GPUTextureFormat enum values: 

"etc2-rgb8unorm" 

"etc2-rgb8unorm-srgb" 

"etc2-rgb8a1unorm" 

"etc2-rgb8a1unorm-srgb" 

"etc2-rgba8unorm" 

"etc2-rgba8unorm-srgb" 

"eac-r11unorm" 

"eac-r11snorm" 

"eac-rg11unorm" 

"eac-rg11snorm" 

25.7. "texture-compression-astc" 

Allows for explicit creation of textures of ASTC compressed formats . Only supports 2D textures. 

This feature adds the following optional API surfaces : 

New GPUTextureFormat enum values: 

"astc-4x4-unorm" 

"astc-4x4-unorm-srgb" 

"astc-5x4-unorm" 

"astc-5x4-unorm-srgb" 

"astc-5x5-unorm" 

"astc-5x5-unorm-srgb" 

"astc-6x5-unorm" 

"astc-6x5-unorm-srgb" 

"astc-6x6-unorm" 

"astc-6x6-unorm-srgb" 

"astc-8x5-unorm" 

"astc-8x5-unorm-srgb" 

"astc-8x6-unorm" 

"astc-8x6-unorm-srgb" 

"astc-8x8-unorm" 

"astc-8x8-unorm-srgb" 

"astc-10x5-unorm" 

"astc-10x5-unorm-srgb" 

"astc-10x6-unorm" 

"astc-10x6-unorm-srgb" 

"astc-10x8-unorm" 

"astc-10x8-unorm-srgb" 

"astc-10x10-unorm" 

"astc-10x10-unorm-srgb" 

"astc-12x10-unorm" 

"astc-12x10-unorm-srgb" 

"astc-12x12-unorm" 

"astc-12x12-unorm-srgb" 

25.8. "texture-compression-astc-sliced-3d" 

Allows the 3d dimension for textures with ASTC compressed formats . 

Note: Adapters which support "texture-compression-astc" do not
always support "texture-compression-astc-sliced-3d" .
To use "texture-compression-astc-sliced-3d" ,
"texture-compression-astc" must be enabled explicitly as this feature
does not enable the ASTC formats. 

This feature adds no optional API surfaces . 

25.9. "timestamp-query" 

Adds the ability to query timestamps from GPU command buffers. See § 20.4 Timestamp Query . 

This feature adds the following optional API surfaces : 

New GPUQueryType values: 

"timestamp" 

New GPUComputePassDescriptor members: 

timestampWrites 

New GPURenderPassDescriptor members: 

timestampWrites 

25.10. "indirect-first-instance" 

Allows the use of non-zero firstInstance values in indirect draw parameters and indirect drawIndexed parameters . 

This feature adds no optional API surfaces . 

25.11. "shader-f16" 

Allows the use of the half-precision floating-point type f16 in WGSL. 

This feature adds the following optional API surfaces : 

New WGSL extensions: 

f16 

25.12. "rg11b10ufloat-renderable" 

Allows the RENDER_ATTACHMENT usage on textures with format "rg11b10ufloat" ,
and also allows textures of that format to be blended, multisampled, and resolved. 

Implicitly allows "rg11b10ufloat" as a destination format in copyExternalImageToTexture() . 

This feature adds no optional API surfaces . 

Note: This feature is automatically enabled by "texture-formats-tier1" ,
which is automatically enabled by "texture-formats-tier2" . 

25.13. "bgra8unorm-storage" 

Allows the STORAGE_BINDING usage on textures with format "bgra8unorm" . 

This feature adds no optional API surfaces . 

25.14. "float32-filterable" 

Makes textures with formats "r32float" , "rg32float" , and
"rgba32float" filterable . 

25.15. "float32-blendable" 

Makes textures with formats "r32float" , "rg32float" , and
"rgba32float" blendable . 

25.16. "clip-distances" 

Allows the use of clip_distances in WGSL. 

This feature adds the following optional API surfaces : 

New WGSL extensions: 

clip_distances 

25.17. "dual-source-blending" 

Allows the use of blend_src in WGSL and simultaneously using both pixel shader outputs
( @blend_src(0) and @blend_src(1) ) as inputs to a blending operation with the single color
attachment at location 0 . 

This feature adds the following optional API surfaces : 

Allows the use of the below GPUBlendFactor s: 

"src1" 

"one-minus-src1" 

"src1-alpha" 

"one-minus-src1-alpha" 

New WGSL extensions: 

dual_source_blending 

25.18. "subgroups" 

Allows the use of the subgroup and quad operations in WGSL. 

This feature adds no optional API surfaces , but the following entries of GPUAdapterInfo 
expose real values whenever the feature is available on the adapter: 

subgroupMinSize 

subgroupMaxSize 

New WGSL extensions: 

subgroups 

25.19. "texture-formats-tier1" 

Enabling "texture-formats-tier1" at device creation will enable
"rg11b10ufloat-renderable" . The following items are in addition to that. 

Supports the below new GPUTextureFormat s with the RENDER_ATTACHMENT ,
blendable , multisampling capabilities and the STORAGE_BINDING capability
with the "read-only" and "write-only" 
GPUStorageTextureAccess es: 

"r16unorm" 

"r16snorm" 

"rg16unorm" 

"rg16snorm" 

"rgba16unorm" 

"rgba16snorm" 

Supports the RENDER_ATTACHMENT , blendable , multisampling and resolve 
capabilities on below GPUTextureFormat s: 

"r8snorm" 

"rg8snorm" 

"rgba8snorm" 

Supports the STORAGE_BINDING capability with the "read-only" and
"write-only" GPUStorageTextureAccess es on below GPUTextureFormat s: 

"r8unorm" 

"r8snorm" 

"r8uint" 

"r8sint" 

"rg8unorm" 

"rg8snorm" 

"rg8uint" 

"rg8sint" 

"r16uint" 

"r16sint" 

"r16float" 

"rg16uint" 

"rg16sint" 

"rg16float" 

"rgb10a2uint" 

"rgb10a2unorm" 

"rg11b10ufloat" 

Implicitly allows the following new destination formats in copyExternalImageToTexture() : 

"r16unorm" 

"rg16unorm" 

"rgba16unorm" 

Note: This feature is automatically enabled by "texture-formats-tier2" . 

25.20. "texture-formats-tier2" 

Enabling "texture-formats-tier2" at device creation will enable
"texture-formats-tier1" . The following items are in addition to that. 

Allows the "read-write" GPUStorageTextureAccess on below
GPUTextureFormat s: 

"r8unorm" 

"r8uint" 

"r8sint" 

"rgba8unorm" 

"rgba8uint" 

"rgba8sint" 

"r16uint" 

"r16sint" 

"r16float" 

"rgba16uint" 

"rgba16sint" 

"rgba16float" 

"rgba32uint" 

"rgba32sint" 

"rgba32float" 

25.21. "primitive-index" 

Allows the use of primitive_index in WGSL. 

This feature adds the following optional API surfaces : 

New WGSL extensions: 

primitive_index 

25.22. "texture-component-swizzle" 

Allows GPUTextureView s to rearrange or replace the color components from texture’s red/green/blue/alpha channels
when used as a TEXTURE_BINDING . 

Also defines previously-implementation-defined behavior when § 26.1.2.1 Reading and Sampling Depth/Stencil Textures . 

This feature adds the following optional API surfaces : 

New GPUTextureViewDescriptor dictionary members: 

swizzle 

26. Appendices 

26.1. Texture Format Capabilities 

26.1.1. Plain color formats 

All supported plain color formats support usages
COPY_SRC , COPY_DST , and
TEXTURE_BINDING , and dimension "3d" . 

The RENDER_ATTACHMENT and STORAGE_BINDING columns
specify support for GPUTextureUsage.RENDER_ATTACHMENT 
and GPUTextureUsage.STORAGE_BINDING usage respectively. 

The render target pixel byte cost 
and render target component alignment 
are used to validate the maxColorAttachmentBytesPerSample limit. 

Note: 
The texel block memory cost of each of these formats is the same as its
texel block copy footprint . 

Format

Required Feature 

GPUTextureSampleType 

RENDER_ATTACHMENT 

blendable 

multisampling 

resolve 

STORAGE_BINDING 

Texel block copy footprint (Bytes)

Render target pixel byte cost (Bytes)

"write-only" 

"read-only" 

"read-write" 

8 bits per component (1-byte render target component alignment )

r8unorm 

"float" ,
"unfilterable-float" 

✓

✓

✓

✓

If "texture-formats-tier1" is enabled

If "texture-formats-tier2" is enabled

1

r8snorm 

"float" ,
"unfilterable-float" 

If "texture-formats-tier1" is enabled

1

r8uint 

"uint" 

✓

If "core-features-and-limits" is enabled

If "texture-formats-tier1" is enabled

If "texture-formats-tier2" is enabled

1

r8sint 

"sint" 

✓

If "core-features-and-limits" is enabled

If "texture-formats-tier1" is enabled

If "texture-formats-tier2" is enabled

1

rg8unorm 

"float" ,
"unfilterable-float" 

✓

✓

✓

✓

If "texture-formats-tier1" is enabled

2

rg8snorm 

"float" ,
"unfilterable-float" 

If "texture-formats-tier1" is enabled

2

rg8uint 

"uint" 

✓

If "core-features-and-limits" is enabled

If "texture-formats-tier1" is enabled

2

rg8sint 

"sint" 

✓

If "core-features-and-limits" is enabled

If "texture-formats-tier1" is enabled

2

rgba8unorm 

"float" ,
"unfilterable-float" 

✓

✓

✓

✓

✓

✓

If "texture-formats-tier2" is enabled

4

8

rgba8unorm-srgb 

"float" ,
"unfilterable-float" 

✓

✓

✓

✓

4

8

rgba8snorm 

"float" ,
"unfilterable-float" 

If "texture-formats-tier1" is enabled

✓

✓

4

8

rgba8uint 

"uint" 

✓

If "core-features-and-limits" is enabled

✓

✓

If "texture-formats-tier2" is enabled

4

rgba8sint 

"sint" 

✓

If "core-features-and-limits" is enabled

✓

✓

If "texture-formats-tier2" is enabled

4

bgra8unorm 

"float" ,
"unfilterable-float" 

✓

✓

✓

✓

If "bgra8unorm-storage" is enabled

4

8

bgra8unorm-srgb 

"core-features-and-limits" 

"float" ,
"unfilterable-float" 

✓

✓

✓

✓

4

8

16 bits per component (2-byte render target component alignment )

r16unorm 

"texture-formats-tier1" 

"unfilterable-float" 

✓

✓

✓

✓

✓

2

r16snorm 

"texture-formats-tier1" 

"unfilterable-float" 

✓

✓

✓

✓

✓

2

r16uint 

"uint" 

✓

If "core-features-and-limits" is enabled

If "texture-formats-tier1" is enabled

If "texture-formats-tier2" is enabled

2

r16sint 

"sint" 

✓

If "core-features-and-limits" is enabled

If "texture-formats-tier1" is enabled

If "texture-formats-tier2" is enabled

2

r16float 

"float" ,
"unfilterable-float" 

✓

✓

✓

✓

If "texture-formats-tier1" is enabled

If "texture-formats-tier2" is enabled

2

rg16unorm 

"texture-formats-tier1" 

"unfilterable-float" 

✓

✓

✓

✓

✓

4

rg16snorm 

"texture-formats-tier1" 

"unfilterable-float" 

✓

✓

✓

✓

✓

4

rg16uint 

"uint" 

✓

If "core-features-and-limits" is enabled

If "texture-formats-tier1" is enabled

4

rg16sint 

"sint" 

✓

If "core-features-and-limits" is enabled

If "texture-formats-tier1" is enabled

4

rg16float 

"float" ,
"unfilterable-float" 

✓

✓

✓

✓

If "texture-formats-tier1" is enabled

4

rgba16unorm 

"texture-formats-tier1" 

"unfilterable-float" 

✓

✓

✓

✓

✓

8

rgba16snorm 

"texture-formats-tier1" 

"unfilterable-float" 

✓

✓

✓

✓

✓

8

rgba16uint 

"uint" 

✓

If "core-features-and-limits" is enabled

✓

✓

If "texture-formats-tier2" is enabled

8

rgba16sint 

"sint" 

✓

If "core-features-and-limits" is enabled

✓

✓

If "texture-formats-tier2" is enabled

8

rgba16float 

"float" ,
"unfilterable-float" 

✓

✓

If "core-features-and-limits" is enabled

✓

✓

If "texture-formats-tier2" is enabled

8

32 bits per component (4-byte render target component alignment )

r32uint 

"uint" 

✓

✓

✓

✓

4

r32sint 

"sint" 

✓

✓

✓

✓

4

r32float 

"float" if "float32-filterable" is enabled

"unfilterable-float" 

✓

If "float32-blendable" is enabled

If "core-features-and-limits" is enabled

✓

✓

✓

4

rg32uint 

"uint" 

✓

If "core-features-and-limits" is enabled

8

rg32sint 

"sint" 

✓

If "core-features-and-limits" is enabled

8

rg32float 

"float" if "float32-filterable" is enabled

"unfilterable-float" 

✓

If "float32-blendable" is enabled

If "core-features-and-limits" is enabled

8

rgba32uint 

"uint" 

✓

✓

✓

If "texture-formats-tier2" is enabled

16

rgba32sint 

"sint" 

✓

✓

✓

If "texture-formats-tier2" is enabled

16

rgba32float 

"float" if "float32-filterable" is enabled

"unfilterable-float" 

✓

If "float32-blendable" is enabled

✓

✓

If "texture-formats-tier2" is enabled

16

mixed component width, 32 bits per texel (4-byte render target component alignment )

rgb10a2uint 

"uint" 

✓

If "core-features-and-limits" is enabled

If "texture-formats-tier1" is enabled

4

8

rgb10a2unorm 

"float" ,
"unfilterable-float" 

✓

✓

✓

✓

If "texture-formats-tier1" is enabled

4

8

rg11b10ufloat 

"float" ,
"unfilterable-float" 

If "rg11b10ufloat-renderable" is enabled

If "texture-formats-tier1" is enabled

4

8

26.1.2. Depth-stencil formats 

A depth-or-stencil format is any format with depth and/or stencil aspects.
A combined depth-stencil format is a depth-or-stencil format that has both
depth and stencil aspects. 

All depth-or-stencil formats support the COPY_SRC , COPY_DST ,
TEXTURE_BINDING , and RENDER_ATTACHMENT usages.
All of these formats support multisampling.
However, certain copy operations also restrict the source and destination formats, and none of
these formats support textures with "3d" dimension. 

Depth textures cannot be used with "filtering" samplers, but can always
be used with "comparison" samplers even if they use filtering. 

Format

NOTE: 

Texel block memory cost (Bytes)

Aspect

GPUTextureSampleType 

Valid texel copy source

Valid texel copy destination

Texel block copy footprint (Bytes)

Aspect-specific format 

stencil8 

1 − 4

stencil

"uint" 

✓

1

stencil8 

depth16unorm 

2

depth

"depth" , "unfilterable-float" 

✓

2

depth16unorm 

depth24plus 

4

depth

"depth" , "unfilterable-float" 

✗

–

depth24plus 

depth24plus-stencil8 

4 − 8

depth

"depth" , "unfilterable-float" 

✗

–

depth24plus 

stencil

"uint" 

✓

1

stencil8 

depth32float 

4

depth

"depth" , "unfilterable-float" 

✓

✗

4

depth32float 

depth32float-stencil8 

5 − 8

depth

"depth" , "unfilterable-float" 

✓

✗

4

depth32float 

stencil

"uint" 

✓

1

stencil8 

24-bit depth refers to a 24-bit unsigned normalized depth format with a range from
0.0 to 1.0, which would be spelled "depth24unorm" if exposed. 

26.1.2.1. Reading and Sampling Depth/Stencil Textures 

It is possible to bind a depth-aspect GPUTextureView 
to either a texture_depth_* binding or a binding with other non-depth 2d/cube texture types. 

A stencil-aspect GPUTextureView must be bound to a normal texture binding type.
The sampleType in the GPUBindGroupLayout 
must be "uint" . 

If the "texture-component-swizzle" feature is enabled, reading or sampling the
depth or stencil aspect of a texture behaves as if the texture contains the values (V, 0, 0, 1) 
where V is the actual depth or stencil value. Otherwise, the values are (V, X, X, X) where each X
is an implementation-defined unspecified value. 

To reduce compatibility issues in practice, implementations should provide (V, 0, 0, 1) 
wherever possible, even if the "texture-component-swizzle" feature is not
enabled. 

For depth-aspect bindings, the unspecified values are not visible through bindings with
texture_depth_* types. 

If a depth texture is bound to tex with type texture_2d<f32> :

textureSample(tex, ...) will return vec4<f32>(D, X, X, X) . 

textureGather(0, tex, ...) will return vec4<f32>(D1, D2, D3, D4) . 

textureGather(2, tex, ...) will return vec4<f32>(X1, X2, X3, X4) (a completely unspecified value). 

Note: 
Short of adding a new more constrained stencil sampler type (like depth), it’s infeasible for
implementations to efficiently paper over the driver differences for depth/stencil reads.
As this was not a portability pain point for WebGL, it’s not expected to be problematic in WebGPU.
In practice, expect either (V, V, V, V) or (V, 0, 0, 1) (where V is the depth or stencil
value), depending on hardware. 

26.1.2.2. Copying Depth/Stencil Textures 

The depth aspects of depth32float formats
( "depth32float" and "depth32float-stencil8" 
have a limited range.
As a result, copies into such textures are only valid from other textures of the same format. 

The depth aspects of depth24plus formats
( "depth24plus" and "depth24plus-stencil8" )
have opaque representations (implemented as either 24-bit depth or "depth32float" ).
As a result, depth-aspect texel copies are not allowed with these formats. 

NOTE: 

It is possible to imitate these disallowed copies:

All of these formats can be written in a render pass using a fragment shader that outputs
depth values via the frag_depth output. 

Textures with "depth24plus" formats can be read as shader textures, and
written to a texture (as a render pass attachment) or
buffer (via a storage buffer binding in a compute shader). 

26.1.3. Packed formats 

All packed texture formats support COPY_SRC , COPY_DST ,
and TEXTURE_BINDING usages.
All of these formats are filterable .
None of these formats are renderable or support multisampling. 

A compressed format is any format with a block size greater than 1×1. 

Note: 
The texel block memory cost of each of these formats is the same as its
texel block copy footprint . 

Format

Texel block copy footprint (Bytes)

GPUTextureSampleType 

Texel block width / height 

"3d" 

Feature 

rgb9e5ufloat 

4

"float" ,
"unfilterable-float" 

1 × 1

✓

bc1-rgba-unorm 

8

"float" ,
"unfilterable-float" 

4 × 4

If "texture-compression-bc-sliced-3d" is enabled

texture-compression-bc 

bc1-rgba-unorm-srgb 

bc2-rgba-unorm 

16

bc2-rgba-unorm-srgb 

bc3-rgba-unorm 

16

bc3-rgba-unorm-srgb 

bc4-r-unorm 

8

bc4-r-snorm 

bc5-rg-unorm 

16

bc5-rg-snorm 

bc6h-rgb-ufloat 

16

bc6h-rgb-float 

bc7-rgba-unorm 

16

bc7-rgba-unorm-srgb 

etc2-rgb8unorm 

8

"float" ,
"unfilterable-float" 

4 × 4

texture-compression-etc2 

etc2-rgb8unorm-srgb 

etc2-rgb8a1unorm 

8

etc2-rgb8a1unorm-srgb 

etc2-rgba8unorm 

16

etc2-rgba8unorm-srgb 

eac-r11unorm 

8

eac-r11snorm 

eac-rg11unorm 

16

eac-rg11snorm 

astc-4x4-unorm 

16

"float" ,
"unfilterable-float" 

4 × 4

If "texture-compression-astc-sliced-3d" is enabled

texture-compression-astc 

astc-4x4-unorm-srgb 

astc-5x4-unorm 

16

5 × 4

astc-5x4-unorm-srgb 

astc-5x5-unorm 

16

5 × 5

astc-5x5-unorm-srgb 

astc-6x5-unorm 

16

6 × 5

astc-6x5-unorm-srgb 

astc-6x6-unorm 

16

6 × 6

astc-6x6-unorm-srgb 

astc-8x5-unorm 

16

8 × 5

astc-8x5-unorm-srgb 

astc-8x6-unorm 

16

8 × 6

astc-8x6-unorm-srgb 

astc-8x8-unorm 

16

8 × 8

astc-8x8-unorm-srgb 

astc-10x5-unorm 

16

10 × 5

astc-10x5-unorm-srgb 

astc-10x6-unorm 

16

10 × 6

astc-10x6-unorm-srgb 

astc-10x8-unorm 

16

10 × 8

astc-10x8-unorm-srgb 

astc-10x10-unorm 

16

10 × 10

astc-10x10-unorm-srgb 

astc-12x10-unorm 

16

12 × 10

astc-12x10-unorm-srgb 

astc-12x12-unorm 

16

12 × 12

astc-12x12-unorm-srgb

---

# WebGPU API - Web APIs | MDN
Source: https://developer.mozilla.org/en-US/docs/Web/API/WebGPU_API

WebGPU API 

Limited availability 

This feature is not Baseline because it does not work in some of the most widely-used browsers. 

Learn more

See full compatibility

Report feedback

Secure context: This feature is available only in secure contexts (HTTPS), in some or all supporting browsers . 

The WebGPU API enables web developers to use the underlying system's GPU (Graphics Processing Unit) to carry out high-performance computations and draw complex images that can be rendered in the browser. 

WebGPU is the successor to WebGL , providing better compatibility with modern GPUs, support for general-purpose GPU computations, faster operations, and access to more advanced GPU features. 

Concepts and usage 

It is fair to say that WebGL revolutionized the web in terms of graphical capabilities after it first appeared around 2011. WebGL is a JavaScript port of the OpenGL ES 2.0 graphics library, allowing web pages to pass rendering computations directly to the device's GPU to be processed at very high speeds, and render the result inside a <canvas> element. 

WebGL and the GLSL language used to write WebGL shader code are complex, so several WebGL libraries have been created to make WebGL apps easier to write: Popular examples include Three.js , Babylon.js , and PlayCanvas . Developers have used these tools to build immersive web-based 3D games, music videos, training and modeling tools, VR and AR experiences, and more. 

However, WebGL has some fundamental issues that needed addressing: 

Since WebGL's release, a new generation of native GPU APIs have appeared — the most popular being Microsoft's Direct3D 12 , Apple's Metal , and The Khronos Group's Vulkan — which provide a multitude of new features. There are no more updates planned to OpenGL (and therefore WebGL), so it won't get any of these new features. WebGPU on the other hand will have new features added to it going forwards. 

WebGL is based wholly around the use case of drawing graphics and rendering them to a canvas. It does not handle general-purpose GPU (GPGPU) computations very well. GPGPU computations are becoming more and more important for many different use cases, for example those based on machine learning models. 

3D graphics apps are becoming increasingly demanding, both in terms of the number of objects to be rendered simultaneously, and usage of new rendering features. 

WebGPU addresses these issues, providing an updated general-purpose architecture compatible with modern GPU APIs, which feels more "webby". It supports graphic rendering, but also has first-class support for GPGPU computations. Rendering of individual objects is significantly cheaper on the CPU side, and it supports modern GPU rendering features such as compute-based particles and post-processing filters like color effects, sharpening, and depth-of-field simulation. In addition, it can handle expensive computations such as culling and skinned model transformation directly on the GPU. 

General model 

There are several layers of abstraction between a device GPU and a web browser running the WebGPU API. It is useful to understand these as you begin to learn WebGPU: 

Physical devices have GPUs. Most devices only have one GPU, but some have more than one. Different GPU types are available: 

Integrated GPUs, which live on the same board as the CPU and share its memory. 

Discrete GPUs, which live on their own board, separate from the CPU. 

Software "GPUs", implemented on the CPU. 

Note: 
The above diagram assumes a device with only one GPU. 

A native GPU API, which is part of the OS (e.g., Metal on macOS), is a programming interface allowing native applications to use the capabilities of the GPU. API instructions are sent to the GPU (and responses received) via a driver. It is possible for a system to have multiple native OS APIs and drivers available to communicate with the GPU, although the above diagram assumes a device with only one native API/driver. 

A browser's WebGPU implementation handles communicating with the GPU via a native GPU API driver. A WebGPU adapter effectively represents a physical GPU and driver available on the underlying system, in your code. 

A logical device is an abstraction via which a single web app can access GPU capabilities in a compartmentalized way. Logical devices are required to provide multiplexing capabilities. A physical device's GPU is used by many applications and processes concurrently, including potentially many web apps. Each web app needs to be able to access WebGPU in isolation for security and logic reasons. 

Accessing a device 

A logical device — represented by a GPUDevice object instance — is the basis from which a web app accesses all WebGPU functionality. Accessing a device is done as follows: 

The Navigator.gpu property (or WorkerNavigator.gpu if you are using WebGPU functionality from inside a worker) returns the GPU object for the current context. 

You access an adapter via the GPU.requestAdapter() method. This method accepts an optional settings object allowing you to request for example a high-performance or low-energy adapter. If this is not included, the device will provide access to the default adapter, which is good enough for most purposes. 

A device can be requested via GPUAdapter.requestDevice() . This method also accepts an options object (referred to as a descriptor), which can be used to specify the exact features and limits you want the logical device to have. If this is not included, the supplied device will have a reasonable general-purpose spec that is good enough for most purposes. 

Putting this together with some feature detection checks, the above process could be achieved as follows: 

js 
async function init() {
if (!navigator.gpu) {
throw Error("WebGPU not supported.");
}

let adapter;
try {
adapter = await navigator.gpu.requestAdapter();
} catch (error) {
console.error(error);
}
if (!adapter) {
throw Error("Couldn't request WebGPU adapter.");
}

const device = await adapter.requestDevice();

// …
}

Pipelines and shaders: WebGPU app structure 

A pipeline is a logical structure containing programmable stages that are completed to get your program's work done. WebGPU is currently able to handle two types of pipeline: 

A render pipeline renders graphics, typically into a <canvas> element, but it could also render graphics offscreen. It has two main stages: 

A vertex stage, in which a vertex shader takes positioning data fed into the GPU and uses it to position a series of vertices in 3D space by applying specified effects like rotation, translation, or perspective. The vertices are then assembled into primitives such as triangles (the basic building block of rendered graphics) and rasterized by the GPU to figure out what pixels each one should cover on the drawing canvas. 

A fragment stage, in which a fragment shader computes the color for each pixel covered by the primitives produced by the vertex shader. These computations frequently use inputs such as images (in the form of textures) that provide surface details and the position and color of virtual lights. 

A compute pipeline is for general computation. A compute pipeline contains a single compute stage in which a compute shader takes general data, processes it in parallel across a specified number of workgroups, then returns the result in one or more buffers. The buffers can contain any kind of data. 

The shaders mentioned above are sets of instructions processed by the GPU. WebGPU shaders are written in a low-level Rust-like language called WebGPU Shading Language (WGSL). 

There are several different ways in which you could architect a WebGPU app, but the process will likely contain the following steps: 

Create shader modules : Write your shader code in WGSL and package it into one or more shader modules. 

Get and configure the canvas context : Get the webgpu context of a <canvas> element and configure it to receive information on what graphics to render from your GPU logical device. This step is not necessary if your app has no graphical output, such as one that only uses compute pipelines. 

Create resources containing your data : The data that you want processed by your pipelines needs to be stored in GPU buffers or textures to be accessed by your app. 

Create pipelines : Define pipeline descriptors that describe the desired pipelines in detail, including the required data structure, bindings, shaders, and resource layouts, then create pipelines from them. Our basic demos only contain a single pipeline, but non-trivial apps will usually contain multiple pipelines for different purposes. 

Run a compute/rendering pass : This involves a number of substeps:

Create a command encoder that can encode a set of commands to be passed to the GPU to execute. 

Create a pass encoder object on which compute/render commands are issued. 

Run commands to specify which pipelines to use, what buffer(s) to get the required data from, how many drawing operations to run (in the case of render pipelines), etc. 

Finalize the command list and encapsulate it in a command buffer. 

Submit the command buffer to the GPU via the logical device's command queue. 

In the sections below, we will examine a basic render pipeline demo, to allow you to explore what it requires. Later on, we'll also examine a basic compute pipeline example, looking at how it differs from the render pipeline. 

Basic render pipeline 

In our basic render demo we give a <canvas> element a solid blue background and draw a triangle onto it. 

Create shader modules 

We are using the following shader code. The vertex shader stage ( @vertex block) accepts a chunk of data containing a position and a color, positions the vertex according to the given position, interpolates the color, then passes the data along to the fragment shader stage. The fragment shader stage ( @fragment block) accepts the data from the vertex shader stage and colors the vertex according to the given color. 

js 
const shaders = `
struct VertexOut {
@builtin(position) position : vec4f,
@location(0) color : vec4f
}

@vertex
fn vertex_main(@location(0) position: vec4f,
@location(1) color: vec4f) -> VertexOut
{
var output : VertexOut;
output.position = position;
output.color = color;
return output;
}

@fragment
fn fragment_main(fragData: VertexOut) -> @location(0) vec4f
{
return fragData.color;
}
`;

Note: 
In our demos we are storing our shader code inside a template literal, but you can store it anywhere from which it can easily be retrieved as text to be fed into your WebGPU program. For example, another common practice is to store shaders inside a <script> element and retrieve the contents using Node.textContent . The correct mime type to use for WGSL is text/wgsl . 

To make your shader code available to WebGPU, you have to put it inside a GPUShaderModule via a GPUDevice.createShaderModule() call, passing your shader code as a property inside a descriptor object. For example: 

js 
const shaderModule = device.createShaderModule({
code: shaders,
});

Get and configure the canvas context 

In a render pipeline, we need to specify somewhere to render the graphics to. In this case we are getting a reference to an onscreen <canvas> element then calling HTMLCanvasElement.getContext() with a parameter of webgpu to return its GPU context (a GPUCanvasContext instance). 

From there, we configure the context with a call to GPUCanvasContext.configure() , passing it an options object containing the GPUDevice that the rendering information will come from, the format the textures will have, and the alpha mode to use when rendering semi-transparent textures. 

js 
const canvas = document.querySelector("#gpuCanvas");
const context = canvas.getContext("webgpu");

context.configure({
device,
format: navigator.gpu.getPreferredCanvasFormat(),
alphaMode: "premultiplied",
});

Note: 
The best practice for determining the texture format is to use the GPU.getPreferredCanvasFormat() method; this selects the most efficient format (either bgra8unorm or rgba8unorm ) for the user's device. 

Create a buffer and write our triangle data into it 

Next we will provide our WebGPU program with our data, in a form it can use. Our data is initially provided in a Float32Array , which contains 8 data points for each triangle vertex — X, Y, Z, W for position, and R, G, B, A for color. 

js 
const vertices = new Float32Array([
0.0, 0.6, 0, 1, 1, 0, 0, 1, -0.5, -0.6, 0, 1, 0, 1, 0, 1, 0.5, -0.6, 0, 1, 0,
0, 1, 1,
]);

However, we've got an issue here. We need to get our data into a GPUBuffer . Behind the scenes, this type of buffer is stored in memory very tightly integrated with the GPU's cores to allow for the desired high performance processing. As a side effect, this memory can't be accessed by processes running on the host system, like the browser. 

The GPUBuffer is created via a call to GPUDevice.createBuffer() . We give it a size equal to the length of the vertices array so it can contain all the data, and VERTEX and COPY_DST usage flags to indicate that the buffer will be used as a vertex buffer and the destination of copy operations. 

js 
const vertexBuffer = device.createBuffer({
size: vertices.byteLength, // make it big enough to store vertices in
usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST,
});

We could handle getting our data into the GPUBuffer using a mapping operation, like we use in the compute pipeline example to read data from the GPU back to JavaScript. However, in this case we are going to use the handy GPUQueue.writeBuffer() convenience method, which takes as its parameters the buffer to write to, the data source to write from, an offset value for each, and the size of data to write (we've specified the whole length of the array). The browser then works out the most efficient way to handle writing the data. 

js 
device.queue.writeBuffer(vertexBuffer, 0, vertices, 0, vertices.length);

Define and create the render pipeline 

Now we've got our data into a buffer, the next part of the setup is to actually create our pipeline, ready to be used for rendering. 

First of all, we create an object that describes the required layout of our vertex data. This perfectly describes what we saw earlier on in our vertices array and vertex shader stage — each vertex has position and color data. Both are formatted in float32x4 format (which maps to the WGSL vec4<f32> type), and the color data starts at an offset of 16 bytes into each vertex. arrayStride specifies the stride, meaning the number of bytes making up each vertex, and stepMode specifies that the data should be fetched per-vertex. 

js 
const vertexBuffers = [
{
attributes: [
{
shaderLocation: 0, // position
offset: 0,
format: "float32x4",
},
{
shaderLocation: 1, // color
offset: 16,
format: "float32x4",
},
],
arrayStride: 32,
stepMode: "vertex",
},
];

Next, we create a descriptor object that specifies the configuration of our render pipeline stages. For both the shader stages, we specify the GPUShaderModule that the relevant code can be found in ( shaderModule ), and the name of the function that acts as the entry point for each stage. 

In addition, in the case of the vertex shader stage we provide our vertexBuffers object to provide the expected state of our vertex data. And in the case of our fragment shader stage, we provide an array of color target states that indicate the specified rendering format (this matches the format specified in our canvas context config earlier). 

We also specify a primitive object, which in this case just states the type of primitive we will be drawing, and a layout of auto . The layout property defines the layout (structure, purpose, and type) of all the GPU resources (buffers, textures, etc.) used during the execution of the pipeline. In more complex apps, this would take the form of a GPUPipelineLayout object, created using GPUDevice.createPipelineLayout() (you can see an example in our Basic compute pipeline ), which allows the GPU to figure out how to run the pipeline most efficiently ahead of time. However, we are specifying the auto value, which will cause the pipeline to generate an implicit bind group layout based on any bindings defined in the shader code. 

js 
const pipelineDescriptor = {
vertex: {
module: shaderModule,
entryPoint: "vertex_main",
buffers: vertexBuffers,
},
fragment: {
module: shaderModule,
entryPoint: "fragment_main",
targets: [
{
format: navigator.gpu.getPreferredCanvasFormat(),
},
],
},
primitive: {
topology: "triangle-list",
},
layout: "auto",
};

Finally, we can create a GPURenderPipeline based on our pipelineDescriptor object, by passing it in as a parameter to a GPUDevice.createRenderPipeline() method call. 

js 
const renderPipeline = device.createRenderPipeline(pipelineDescriptor);

Running a rendering pass 

Now that all the setup is done, we can actually run a rendering pass and draw something onto our <canvas> . To encode any commands to be later issued to the GPU, you need to create a GPUCommandEncoder instance, which is done using a GPUDevice.createCommandEncoder() call. 

js 
const commandEncoder = device.createCommandEncoder();

Next up we start the rendering pass running by creating a GPURenderPassEncoder instance with a GPUCommandEncoder.beginRenderPass() call. This method takes a descriptor object as a parameter, the only mandatory property of which is a colorAttachments array. In this case, we specify: 

A texture view to render into; we create a new view from the <canvas> via context.getCurrentTexture().createView() . 

That the view should be "cleared" to a specified color once loaded and before any drawing takes place. This is what causes the blue background behind the triangle. 

That the value of the current rendering pass should be stored for this color attachment. 

js 
const clearColor = { r: 0.0, g: 0.5, b: 1.0, a: 1.0 };

const renderPassDescriptor = {
colorAttachments: [
{
clearValue: clearColor,
loadOp: "clear",
storeOp: "store",
view: context.getCurrentTexture().createView(),
},
],
};

const passEncoder = commandEncoder.beginRenderPass(renderPassDescriptor);

Now we can invoke methods of the rendering pass encoder to draw our triangle: 

GPURenderPassEncoder.setPipeline() is called with our renderPipeline object as a parameter to specify the pipeline to use for the rendering pass. 

GPURenderPassEncoder.setVertexBuffer() is called with our vertexBuffer object as a parameter to act as the data source to pass to the pipeline to render. The first parameter is the slot to set the vertex buffer for, and is a reference to the index of the element in the vertexBuffers array which describes this buffer's layout. 

GPURenderPassEncoder.draw() sets the drawing in motion. There is data for three vertices inside our vertexBuffer , so we set a vertex count value of 3 to draw them all. 

js 
passEncoder.setPipeline(renderPipeline);
passEncoder.setVertexBuffer(0, vertexBuffer);
passEncoder.draw(3);

To finish encoding the sequence of commands and issue them to the GPU, three more steps are needed. 

We invoke the GPURenderPassEncoder.end() method to signal the end of the render pass command list. 

We invoke the GPUCommandEncoder.finish() method to complete recording of the issued command sequence and encapsulate it into a GPUCommandBuffer object instance. 

We submit the GPUCommandBuffer to the device's command queue (represented by a GPUQueue instance) to be sent to the GPU. The device's queue is available via the GPUDevice.queue property, and an array of GPUCommandBuffer instances can be added to the queue via a GPUQueue.submit() call. 

These three steps can be achieved via the following two lines: 

js 
passEncoder.end();

device.queue.submit([commandEncoder.finish()]);

Basic compute pipeline 

In our basic compute demo , we get the GPU to calculate some values, store them in an output buffer, copy the data across to a staging buffer, then map that staging buffer so that the data can be read out to JavaScript and logged to the console. 

The app follows a similar structure to the basic rendering demo. We create a GPUDevice reference in the same way as before, and encapsulate our shader code into a GPUShaderModule via a GPUDevice.createShaderModule() call. The difference here is that our shader code only has one shader stage, a @compute stage: 

js 
// Define global buffer size
const NUM_ELEMENTS = 1000;
const BUFFER_SIZE = NUM_ELEMENTS * 4; // Buffer size, in bytes

const shader = `
@group(0) @binding(0)
var<storage, read_write> output: array<f32>;

@compute @workgroup_size(64)
fn main(
@builtin(global_invocation_id)
global_id : vec3u,

@builtin(local_invocation_id)
local_id : vec3u,
) {
// Avoid accessing the buffer out of bounds
if (global_id.x >= ${NUM_ELEMENTS}) {
return;
}

output[global_id.x] =
f32(global_id.x) * 1000. + f32(local_id.x);
}
`;

Create buffers to handle our data 

In this example we create two GPUBuffer instances to handle our data, an output buffer to write the GPU calculation results to at high speed, and a stagingBuffer that we'll copy the output contents to, which can be mapped to allow JavaScript to access the values. 

output is specified as a storage buffer that will be the source of a copy operation. 

stagingBuffer is specified as a buffer that can be mapped for reading by JavaScript, and will be the destination of a copy operation. 

js 
const output = device.createBuffer({
size: BUFFER_SIZE,
usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_SRC,
});

const stagingBuffer = device.createBuffer({
size: BUFFER_SIZE,
usage: GPUBufferUsage.MAP_READ | GPUBufferUsage.COPY_DST,
});

Create a bind group layout 

When the pipeline is created, we specify a bind group to use for the pipeline. This involves first creating a GPUBindGroupLayout (via a call to GPUDevice.createBindGroupLayout() ) that defines the structure and purpose of GPU resources such as buffers that will be used in this pipeline. This layout is used as a template for bind groups to adhere to. In this case we give the pipeline access to a single memory buffer, tied to binding slot 0 (this matches the relevant binding number in our shader code — @binding(0) ), usable in the compute stage of the pipeline, and with the buffer's purpose defined as storage . 

js 
const bindGroupLayout = device.createBindGroupLayout({
entries: [
{
binding: 0,
visibility: GPUShaderStage.COMPUTE,
buffer: {
type: "storage",
},
},
],
});

Next we create a GPUBindGroup by calling GPUDevice.createBindGroup() . We pass this method call a descriptor object that specifies the bind group layout to base this bind group on, and the details of the variable to bind to the slot defined in the layout. In this case, we are declaring binding 0, and specifying that the output buffer we defined earlier should be bound to it. 

js 
const bindGroup = device.createBindGroup({
layout: bindGroupLayout,
entries: [
{
binding: 0,
resource: {
buffer: output,
},
},
],
});

Note: 
You could retrieve an implicit layout to use when creating a bind group by calling the GPUComputePipeline.getBindGroupLayout() method. There is also a version available for render pipelines: see GPURenderPipeline.getBindGroupLayout() . 

Create a compute pipeline 

With the above all in place, we can now create a compute pipeline by calling GPUDevice.createComputePipeline() , passing it a pipeline descriptor object. This works in a similar way to creating a render pipeline. We describe the compute shader, specifying what module to find the code in and what the entry point is. We also specify a layout for the pipeline, in this case creating a layout based on the bindGroupLayout we defined earlier via a GPUDevice.createPipelineLayout() call. 

js 
const computePipeline = device.createComputePipeline({
layout: device.createPipelineLayout({
bindGroupLayouts: [bindGroupLayout],
}),
compute: {
module: shaderModule,
entryPoint: "main",
},
});

One difference here from the render pipeline layout is that we are not specifying a primitive type, as we are not drawing anything. 

Running a compute pass 

Running a compute pass is similar in structure to running a rendering pass, with some different commands. For a start, the pass encoder is created using GPUCommandEncoder.beginComputePass() . 

When issuing the commands, we specify the pipeline to use in the same way as before, using GPUComputePassEncoder.setPipeline() . We then however use GPUComputePassEncoder.setBindGroup() to specify that we want to use our bindGroup to specify the data to use in the calculation, and GPUComputePassEncoder.dispatchWorkgroups() to specify the number of GPU workgroups to use to run the calculations. 

We then signal the end of the render pass command list using GPURenderPassEncoder.end() . 

js 
passEncoder.setPipeline(computePipeline);
passEncoder.setBindGroup(0, bindGroup);
passEncoder.dispatchWorkgroups(Math.ceil(NUM_ELEMENTS / 64));

passEncoder.end();

Reading the results back to JavaScript 

Before submitting the encoded commands to the GPU for execution using GPUQueue.submit() , we copy the contents of the output buffer to the stagingBuffer buffer using GPUCommandEncoder.copyBufferToBuffer() . 

js 
// Copy output buffer to staging buffer
commandEncoder.copyBufferToBuffer(
output,
0, // Source offset
stagingBuffer,
0, // Destination offset
BUFFER_SIZE, // Length, in bytes
);

// End frame by passing array of command buffers to command queue for execution
device.queue.submit([commandEncoder.finish()]);

Once the output data is available in the stagingBuffer , we use the GPUBuffer.mapAsync() method to map the data to intermediate memory, grab a reference to the mapped range using GPUBuffer.getMappedRange() , copy the data into JavaScript, and then log it to the console. We also unmap the stagingBuffer once we are finished with it. 

js 
// map staging buffer to read results back to JS
await stagingBuffer.mapAsync(
GPUMapMode.READ,
0, // Offset
BUFFER_SIZE, // Length, in bytes
);

const copyArrayBuffer = stagingBuffer.getMappedRange(0, BUFFER_SIZE);
const data = copyArrayBuffer.slice();
stagingBuffer.unmap();
console.log(new Float32Array(data));

GPU error handling 

WebGPU calls are validated asynchronously in the GPU process. If errors are found, the problem call is marked as invalid on the GPU side. If another call is made that relies on the return value of an invalidated call, that object will also be marked as invalid, and so on. For this reason, errors in WebGPU are referred to as "contagious". 

Each GPUDevice instance maintains its own error scope stack. This stack is initially empty, but you can start pushing an error scope to the stack by invoking GPUDevice.pushErrorScope() to capture errors of a particular type. 

Once you are done capturing errors, you can end capture by invoking GPUDevice.popErrorScope() . This pops the scope from the stack and returns a Promise that resolves to an object ( GPUInternalError , GPUOutOfMemoryError , or GPUValidationError ) describing the first error captured in the scope, or null if no errors were captured. 

We have attempted to provide useful information to help you understand why errors are occurring in your WebGPU code in "Validation" sections where appropriate, which list criteria to meet to avoid errors. See for example the GPUDevice.createBindGroup() Validation section . Some of this information is complex; rather than repeat the spec, we have decided to just list error criteria that are: 

Non-obvious, for example combinations of descriptor properties that produce validation errors. There is no point telling you to make sure you use the correct descriptor object structure. That is both obvious and vague. 

Developer-controlled. Some of the error criteria are purely based on internals and not really relevant to web developers. 

You can find more information about WebGPU error handling in the explainer — see Object validity and destroyed-ness and Errors . WebGPU Error Handling best practices provides useful real-world examples and advice. 

Note: 
The historic way of handling errors in WebGL is to provide a getError() method to return error information. This is problematic in that it returns errors synchronously, which is bad for performance — each call requires a round-trip to the GPU and requires all previously issued operations to be finished. Its state model is also flat, meaning that errors can leak between unrelated code. The creators of WebGPU were determined to improve on this. 

Interfaces 

Entry point for the API 

Navigator.gpu / WorkerNavigator.gpu 

The entry point for the API — returns the GPU object for the current context. 

GPU 

The starting point for using WebGPU. It can be used to return a GPUAdapter . 

GPUAdapter 

Represents a GPU adapter. From this you can request a GPUDevice , adapter info, features, and limits. 

GPUAdapterInfo 

Contains identifying information about an adapter. 

Configuring GPUDevices 

GPUDevice 

Represents a logical GPU device. This is the main interface through which the majority of WebGPU functionality is accessed. 

GPUSupportedFeatures 

A setlike object that describes additional functionality supported by a GPUAdapter or GPUDevice . 

GPUSupportedLimits 

Describes the limits supported by a GPUAdapter or GPUDevice . 

Configuring a rendering <canvas> 

HTMLCanvasElement.getContext() — the "webgpu" contextType 

Invoking getContext() with the "webgpu" contextType returns a GPUCanvasContext object instance, which can then be configured with GPUCanvasContext.configure() . 

GPUCanvasContext 

Represents the WebGPU rendering context of a <canvas> element. 

Representing pipeline resources 

GPUBuffer 

Represents a block of memory that can be used to store raw data to use in GPU operations. 

GPUExternalTexture 

A wrapper object containing an HTMLVideoElement snapshot that can be used as a texture in GPU rendering operations. 

GPUSampler 

Controls how shaders transform and filter texture resource data. 

GPUShaderModule 

A reference to an internal shader module object, a container for WGSL shader code that can be submitted to the GPU to execution by a pipeline. 

GPUTexture 

A container used to store 1D, 2D, or 3D arrays of data, such as images, to use in GPU rendering operations. 

GPUTextureView 

A view onto some subset of the texture subresources defined by a particular GPUTexture . 

Representing pipelines 

GPUBindGroup 

Based on a GPUBindGroupLayout , a GPUBindGroup defines a set of resources to be bound together in a group and how those resources are used in shader stages. 

GPUBindGroupLayout 

Defines the structure and purpose of related GPU resources such as buffers that will be used in a pipeline, and is used as a template when creating GPUBindGroup s. 

GPUComputePipeline 

Controls the compute shader stage and can be used in a GPUComputePassEncoder . 

GPUPipelineLayout 

Defines the GPUBindGroupLayout s used by a pipeline. GPUBindGroup s used with the pipeline during command encoding must have compatible GPUBindGroupLayout s. 

GPURenderPipeline 

Controls the vertex and fragment shader stages and can be used in a GPURenderPassEncoder or GPURenderBundleEncoder . 

Encoding and submitting commands to the GPU 

GPUCommandBuffer 

Represents a recorded list of GPU commands that can be submitted to a GPUQueue for execution. 

GPUCommandEncoder 

Represents a command encoder, used to encode commands to be issued to the GPU. 

GPUComputePassEncoder 

Encodes commands related to controlling the compute shader stage, as issued by a GPUComputePipeline . Part of the overall encoding activity of a GPUCommandEncoder . 

GPUQueue 

controls execution of encoded commands on the GPU. 

GPURenderBundle 

A container for pre-recorded bundles of commands (see GPURenderBundleEncoder ). 

GPURenderBundleEncoder 

Used to pre-record bundles of commands. These can be reused in GPURenderPassEncoder s via the executeBundles() method, as many times as required. 

GPURenderPassEncoder 

Encodes commands related to controlling the vertex and fragment shader stages, as issued by a GPURenderPipeline . Part of the overall encoding activity of a GPUCommandEncoder . 

Running queries on rendering passes 

GPUQuerySet 

Used to record the results of queries on passes, such as occlusion or timestamp queries. 

Debugging errors 

GPUCompilationInfo 

An array of GPUCompilationMessage objects, generated by the GPU shader module compiler to help diagnose problems with shader code. 

GPUCompilationMessage 

Represents a single informational, warning, or error message generated by the GPU shader module compiler. 

GPUDeviceLostInfo 

Returned when the GPUDevice.lost Promise resolves, providing information as to why the device was lost. 

GPUError 

The base interface for errors surfaced by GPUDevice.popErrorScope and the uncapturederror event. 

GPUInternalError 

One of the types of errors surfaced by GPUDevice.popErrorScope and the GPUDevice uncapturederror event. Indicates that an operation failed for a system or implementation-specific reason, even when all validation requirements were satisfied. 

GPUOutOfMemoryError 

One of the types of errors surfaced by GPUDevice.popErrorScope and the GPUDevice uncapturederror event. Indicates that there was not enough free memory to complete the requested operation. 

GPUPipelineError 

Describes a pipeline failure. The value received when a Promise returned by a GPUDevice.createComputePipelineAsync() or GPUDevice.createRenderPipelineAsync() call rejects. 

GPUUncapturedErrorEvent 

The event object type for the GPUDevice uncapturederror event. 

GPUValidationError 

One of the types of errors surfaced by GPUDevice.popErrorScope and the GPUDevice uncapturederror event. Describes an application error indicating that an operation did not pass the WebGPU API's validation constraints. 

Security requirements 

The whole API is available only in a secure context . 

Examples 

Basic compute demo 

Basic render demo 

WebGPU samples 

Specifications 

Specification 

WebGPU 
# gpu-interface 

Browser compatibility 

See also 

WebGPU best practices 

WebGPU explainer 

WebGPU — All of the cores, none of the canvas 

Help improve MDN

Was this page helpful to you? 

Yes 

No 

Learn how to contribute 
This page was last modified on Jan 13, 2026 by MDN contributors .

View this page on GitHub • Report a problem with this content

---

# WebGL - Low-Level 3D Graphics API Based on OpenGL ES
Source: https://khronos.org/webgl/

Related Discussions 

Visit Community Forums 

Visit Khronos Discord 

Visit Vulkan Discord 

Related News 

Google and Red Games Co. Use WebGL to Bring Crayola Create & Play to the Web 

Khronos Group at SIGGRAPH 

Khronos at SIGGRAPH 2025 

When WebGL and Branding Transform the User Experience: The Case of Duroc 

WebGL-Based SuperSplat 2.0 Released 

More news 

Related Press 

Khronos COLLADA now recognized as ISO Standard 

More Press Releases

---

# WebGL: 2D and 3D graphics for the web - Web APIs | MDN
Source: https://developer.mozilla.org/en-US/docs/Web/API/WebGL_API

WebGL: 2D and 3D graphics for the web 

Note: This feature is available in Web Workers . 

WebGL (Web Graphics Library) is a JavaScript API for rendering high-performance interactive 3D and 2D graphics within any compatible web browser without the use of plug-ins. WebGL does so by introducing an API that closely conforms to OpenGL ES 2.0 that can be used in HTML <canvas> elements. This conformance makes it possible for the API to take advantage of hardware graphics acceleration provided by the user's device. 

Support for WebGL is present in all modern browsers (see the compatibility tables below); however, the user's device must also have hardware that supports these features. 

The WebGL 2 API introduces support for much of the OpenGL ES 3.0 feature set; it's provided through the WebGL2RenderingContext interface. 

The <canvas> element is also used by the Canvas API to do 2D graphics on web pages. 

Reference 

Standard interfaces 

WebGLRenderingContext 

WebGL2RenderingContext 

WebGLActiveInfo 

WebGLBuffer 

WebGLContextEvent 

WebGLFramebuffer 

WebGLProgram 

WebGLQuery 

WebGLRenderbuffer 

WebGLSampler 

WebGLShader 

WebGLShaderPrecisionFormat 

WebGLSync 

WebGLTexture 

WebGLTransformFeedback 

WebGLUniformLocation 

WebGLVertexArrayObject 

Extensions 

ANGLE_instanced_arrays 

EXT_blend_minmax 

EXT_color_buffer_float 

EXT_color_buffer_half_float 

EXT_disjoint_timer_query 

EXT_float_blend 
Experimental 

EXT_frag_depth 

EXT_shader_texture_lod 

EXT_sRGB 

EXT_texture_compression_bptc 

EXT_texture_compression_rgtc 

EXT_texture_filter_anisotropic 

EXT_texture_norm16 

KHR_parallel_shader_compile 

OES_draw_buffers_indexed 

OES_element_index_uint 

OES_fbo_render_mipmap 

OES_standard_derivatives 

OES_texture_float 

OES_texture_float_linear 

OES_texture_half_float 

OES_texture_half_float_linear 

OES_vertex_array_object 

OVR_multiview2 

WEBGL_color_buffer_float 

WEBGL_compressed_texture_astc 

WEBGL_compressed_texture_etc 

WEBGL_compressed_texture_etc1 

WEBGL_compressed_texture_pvrtc 

WEBGL_compressed_texture_s3tc 

WEBGL_compressed_texture_s3tc_srgb 

WEBGL_debug_renderer_info 

WEBGL_debug_shaders 

WEBGL_depth_texture 

WEBGL_draw_buffers 

WEBGL_lose_context 

WEBGL_multi_draw 

Events 

webglcontextlost 

webglcontextrestored 

webglcontextcreationerror 

Constants and types 

WebGL constants 

WebGL types 

WebGL 2 

WebGL 2 is a major update to WebGL which is provided through the WebGL2RenderingContext interface. It is based on OpenGL ES 3.0 and new features include: 

3D textures , 

Sampler objects , 

Uniform Buffer objects , 

Sync objects , 

Query objects , 

Transform Feedback objects , 

Promoted extensions that are now core to WebGL 2: Vertex Array objects , instancing , multiple render targets , fragment depth . 

See also the blog post "WebGL 2 lands in Firefox" and webglsamples.org/WebGL2Samples for a few demos. 

Guides and tutorials 

Below, you'll find an assortment of guides to help you learn WebGL concepts and tutorials that offer step-by-step lessons and examples. 

Guides 

Data in WebGL 

A guide to variables, buffers, and other types of data used when writing WebGL code. 

WebGL best practices 

Tips and suggestions to help you improve the quality, performance, and reliability of your WebGL content. 

Using extensions 

A guide to using WebGL extensions. 

Tutorials 

WebGL tutorial 

A beginner's guide to WebGL core concepts. A good place to start if you don't have previous WebGL experience. 

Examples 

A basic 2D WebGL animation example 

This example demonstrates the simple animation of a one-color shape. Topics examined are adapting to aspect ratio differences, a function to build shader programs from sets of multiple shaders, and the basics of drawing in WebGL. 

WebGL by example 

A series of live samples with short explanations that showcase WebGL concepts and capabilities. The examples are sorted according to topic and level of difficulty, covering the WebGL rendering context, shader programming, textures, geometry, user interaction, and more. 

Advanced tutorials 

Compressed texture formats 

How to enable and use compressed texture formats for better memory performance. 

WebGL model view projection 

A detailed explanation of the three core matrices that are typically used to represent a 3D object view: the model, view and projection matrices. 

Matrix math for the web 

A useful guide to how 3D transform matrices work, and can be used on the web — both for WebGL calculations and in CSS transforms. 

Resources 

Khronos WebGL site The main website for WebGL at the Khronos Group. 

WebGL Fundamentals A basic tutorial with fundamentals of WebGL. 

Raw WebGL: An introduction to WebGL A talk by Nick Desaulniers that introduces the basics of WebGL. 

WebGL Academy An HTML/JavaScript editor with tutorials to learn basics of webgl programming. 

WebGL Stats A site with statistics about WebGL capabilities in browsers on different platforms. 

Libraries 

three.js is an open-source, fully featured 3D WebGL library. 

Babylon.js is a powerful, simple, and open game and 3D rendering engine packed into a friendly JavaScript framework. 

Pixi.js is a fast, open-source 2D WebGL renderer. 

Phaser is a fast, free and fun open source framework for Canvas and WebGL powered browser games. 

PlayCanvas is an open-source game engine. 

glMatrix is a JavaScript matrix and vector library for high-performance WebGL apps. 

twgl is a library for making webgl less verbose. 

RedGL is an open-source 3D WebGL library. 

vtk.js is a JavaScript library for scientific visualization in your browser. 

webgl-lint will help find errors in your WebGL code and provide useful info 

Specifications 

Specification 

WebGL Specification 
# 5.14 

WebGL 2.0 Specification 
# 3.7 

Browser compatibility 

api.WebGLRenderingContext 

api.WebGL2RenderingContext 

Compatibility notes 

In addition to the browser, the GPU itself also needs to support the feature. So, for example, S3 Texture Compression (S3TC) is only available on Tegra-based tablets. Most browsers make the WebGL context available through the webgl context name, but older ones need experimental-webgl as well. In addition, the upcoming WebGL 2 is fully backwards-compatible and will have the context name webgl2 . 

Gecko notes 

WebGL debugging and testing 

Firefox provides two preferences available which let you control the capabilities of WebGL for testing purposes: 

webgl.min_capability_mode 

A Boolean property that, when true , enables a minimum capability mode. When in this mode, WebGL is configured to only support the bare minimum feature set and capabilities required by the WebGL specification. This lets you ensure that your WebGL code will work on any device or browser, regardless of their capabilities. This is false by default. 

webgl.disable_extensions 

A Boolean property that, when true , disables all WebGL extensions. This is false by default. 

See also 

Canvas API 

Compatibility info about WebGL extensions 

Help improve MDN

Was this page helpful to you? 

Yes 

No 

Learn how to contribute 
This page was last modified on Jul 15, 2025 by MDN contributors .

View this page on GitHub • Report a problem with this content

---

# Payment Request API - Web APIs | MDN
Source: https://developer.mozilla.org/en-US/docs/Web/API/Payment_Request_API

Payment Request API 

Limited availability 

This feature is not Baseline because it does not work in some of the most widely-used browsers. 

Learn more

See full compatibility

Report feedback

Secure context: This feature is available only in secure contexts (HTTPS), in some or all supporting browsers . 

The Payment Request API provides a consistent user experience for merchants and users. It is not a new way of paying for things; instead, it's a way for users to select their preferred way of paying for things and make that information available to a merchant. 

Concepts and usage 

Many problems related to online shopping cart abandonment can be traced to checkout forms, which can be difficult and time-consuming to fill out and often require multiple steps to complete. The Payment Request API is meant to reduce the steps needed to complete payment online, potentially doing away with checkout forms. It aims to make the checkout process more accessible by having payment apps store a user's details, which are passed along to a merchant, hopefully without requiring an HTML form. 

To request a payment, a web page creates a PaymentRequest object in response to a user action that initiates a payment, such as clicking a "Purchase" button. The PaymentRequest allows the web page to exchange information with the user agent while the user provides input to complete the transaction. 

You can find a complete guide in Using the Payment Request API . 

Note: 
The API is available inside cross-origin <iframe> elements only if they have had the allowpaymentrequest attribute set on them. 

Interfaces 

PaymentAddress 
Deprecated 

Non-standard 

An object that contains address information; used for billing and shipping addresses, for example. 

PaymentRequest 

An object that provides the API for creating and managing the user agent's payment interface. 

PaymentRequestUpdateEvent 

Enables the web page to update the details of the payment request in response to a user action. 

PaymentMethodChangeEvent 

Represents the user changing payment instrument (e.g., switching from one payment method to another). 

PaymentResponse 

An object returned after the user selects a payment method and approves a payment request. 

MerchantValidationEvent 
Deprecated 

Represents the browser requiring the merchant (website) to validate themselves as allowed to use a particular payment handler (e.g., registered as allowed to use Apple Pay). 

Specifications 

Specification 

Payment Request API 
# paymentrequest-interface 

Browser compatibility 

See also 

Using the Payment Request API 

Payment processing concepts 

Introducing the Payment Request API for Apple Pay 

Google Pay API PaymentRequest Tutorial 

Samsung Pay Web Payments Integration Guide 

W3C Payment Request API FAQ 

Permissions Policy directive payment 

Help improve MDN

Was this page helpful to you? 

Yes 

No 

Learn how to contribute 
This page was last modified on Apr 10, 2025 by MDN contributors .

View this page on GitHub • Report a problem with this content

---

# Payment Request API
Source: https://www.w3.org/TR/payment-request/

Payment Request API

W3C Candidate Recommendation Draft 22 March 2026 

More details about this document 

This version: 
https://www.w3.org/TR/2026/CRD-payment-request-20260322/ 

Latest published version: 
https://www.w3.org/TR/payment-request/ 

Latest editor's draft: https://w3c.github.io/payment-request/ 
History: 
https://www.w3.org/standards/history/payment-request/ 

Commit history 

Test suite: https://wpt.live/payment-request/ 
Implementation report: 
https://w3c.github.io/test-results/payment-request/all.html 

Latest Recommendation: https://www.w3.org/TR/2022/REC-payment-request-20220908/ 
Editors: 
Marcos Cáceres ( Apple Inc. )

Ian Jacobs ( W3C )

Stephen McGruer ( Google )

Former editors:

Domenic Denicola ( Google )

Adrian Bateman ( Microsoft Corporation )

Zach Koch ( Google )

Roy McElmurry ( Facebook )

Danyao Wang ( Google )

Rouslan Solomakhin ( Google ) - Until 30 September 2025 

Feedback: 
GitHub w3c/payment-request 
( pull requests ,
new issue ,
open issues )

Browser support: caniuse.com 

Copyright 
©
2026

World Wide Web Consortium .
W3C ® 
liability ,
trademark and
permissive document license rules apply.

Abstract 

This specification standardizes an API to allow merchants (i.e. web
sites selling physical or digital goods) to utilize one or more payment
methods with minimal integration. User agents (e.g., browsers)
facilitate the payment flow between merchant and user.

Status of This Document 
This section describes the status of this
document at the time of its publication. A list of current W3C 
publications and the latest revision of this technical report can be found
in the
W3C standards and drafts index . 

In September 2022 the Web Payments Working Group published a Payment
Request Recommendation . Following privacy and internationalization
reviews, the Recommendation excluded 
capabilities related to billing and shipping addresses. However,
implementations have continued to support those features interoperably,
and so the Working Group has decided to try to re-align the
specification with implementations, and re-engage the community on
associated issues.

This document is a Candidate Recommendation Snapshot based on the text
of the original Recommendation. A subsequent Candidate Recommendation
Draft will add back address capabilities and a small number of other
changes made since publication of the Recommendation.

As part of adding back support for addresses, this specification now
refers to the address components defined in the Contact Picker API rather
than define those components itself. Indeed, the Contact Picker API is
derived from the original definitions found in Payment Request API, and
pulled out of the specification because addresses are useful on the Web
beyond payments.

The Working Group plans to engage in discussion and follow the usual
review process before advancing the specification to Proposed
Recommendation status.

The working group will demonstrate implementation experience by
producing an implementation
report . The report will show two or more independent
implementations passing each mandatory test in the test suite (i.e., each test
corresponds to a MUST requirement of the specification).

This document was published by the Web Payments Working Group as
a Candidate Recommendation Draft using the
Recommendation track . 

Publication as a Candidate Recommendation does not
imply endorsement by W3C and its Members. A Candidate Recommendation Draft integrates
changes from the previous Candidate Recommendation that the Working Group
intends to include in a subsequent Candidate Recommendation Snapshot. 

This is a draft document and may be updated, replaced, or obsoleted by other
documents at any time. It is inappropriate to cite this document as other
than a work in progress.
Future updates to this upcoming Recommendation may incorporate
new features .

This document was produced by a group
operating under the
W3C Patent
Policy .

W3C maintains a
public list of any patent disclosures 
made in connection with the deliverables of
the group; that page also includes
instructions for disclosing a patent. An individual who has actual
knowledge of a patent that the individual believes contains
Essential Claim(s) 
must disclose the information in accordance with
section 6 of the W3C Patent Policy .

This document is governed by the
18 August 2025 W3C Process Document .

1. 
Introduction

This section is non-normative. 

This specification describes an API that allows user agents 
(e.g., browsers) to act as an intermediary between three parties in a
transaction:

The payee: the merchant that runs an online store, or other party
that requests to be paid.

The payer: the party that makes a purchase at that online store,
and who authenticates and authorizes payment as required.

The payment method : the means that the payer uses to pay
the payee (e.g., a card payment or credit transfer). The payment
method provider establishes the ecosystem to support that payment
method.

A payment method defines:

An optional additional data
type 

Optionally, an IDL type that the payment method expects to
receive as the PaymentMethodData 's data 
member. If not specified for a given payment method, no conversion to
IDL is done and the payment method will receive
data as JSON.

Steps to validate payment method data 

Algorithmic steps that specify how a payment method validates
the data member of the PaymentMethodData ,
after it is converted to the payment method's additional data type . If not specified for a given payment
method, no validation is done.

The details of how to fulfill a payment request for a given payment
method is an implementation detail of a payment
handler , which is an application or service that handles requests
for payment. Concretely, a payment handler defines:

Steps to check if a payment can be made :

How a payment handler determines whether it, or the user, can
potentially "make a payment" is also an implementation detail of a
payment handler.

Steps to respond to a payment request :

Steps that return an object or dictionary that a merchant uses
to process or validate the transaction. The structure of this object
is specific to each payment method .

Steps for when a user changes payment method (optional)

Steps that describe how to handle the user changing payment method
or monetary instrument (e.g., from a debit card to a credit card)
that results in a dictionary or object or null.

This API also enables web sites to take advantage of more secure
payment schemes (e.g., tokenization and system-level authentication)
that are not possible with standard JavaScript libraries. This has the
potential to reduce liability for the merchant and helps protect
sensitive user information.

1.1 
Goals and scope

Allow the user agent to act as intermediary between a merchant,
user, and payment method provider .

Enable user agents to streamline the user's payment experience by
taking into account user preferences, merchant information, security
considerations, and other factors.

Standardize (to the extent that it makes sense) the communication
flow between a merchant, user agent, and payment method
provider .

Enable a payment method provider to bring more secure
payment transactions to the web.

The following are out of scope for this specification:

Create a new payment method .

Integrate directly with payment processors.

2. 
Examples of usage

This section is non-normative. 

In order to use the API, the developer needs to provide and keep track
of a number of key pieces of information. These bits of information are
passed to the PaymentRequest constructor as arguments, and
subsequently used to update the payment request being displayed to the
user. Namely, these bits of information are:

The methodData : A sequence of PaymentMethodData s that
represents the payment methods that the site supports (e.g., "we
support card-based payments, but only Visa and MasterCard credit
cards.").

The details : The details of the transaction, as a
PaymentDetailsInit dictionary. This includes total cost, and
optionally a list of goods or services being purchased, for physical
goods, and shipping options. Additionally, it can optionally include
"modifiers" to how payments are made. For example, "if you pay with a
card belonging to network X, it incurs a US$3.00 processing fee".

The options : Optionally, a list of things as PaymentOptions 
that the site needs to deliver the good or service (e.g., for physical
goods, the merchant will typically need a physical address to ship to.
For digital goods, an email will usually suffice).

Once a PaymentRequest is constructed, it's presented to the end
user via the show () method. The
show () returns a promise that, once the user
confirms request for payment, results in a PaymentResponse .

2.1 
Declaring multiple ways of paying

When constructing a new PaymentRequest , a merchant uses the first
argument ( methodData ) to list the different ways a user can pay for
things (e.g., credit cards, Apple Pay, Google Pay, etc.). More
specifically, the methodData sequence contains
PaymentMethodData dictionaries containing the payment
method identifiers for the payment methods that the
merchant accepts and any associated payment method specific
data (e.g., which credit card networks are supported).

Example 1 : The `methodData` argument 

const methodData = [
{
supportedMethods : "https://example.com/payitforward" ,
data : {
payItForwardField : "ABC" ,
},
},
{
supportedMethods : "https://example.com/bobpay" ,
data : {
merchantIdentifier : "XXXX" ,
bobPaySpecificField : true ,
},
},
]; 

2.2 
Describing what is being paid for

When constructing a new PaymentRequest , a merchant uses the
second argument of the constructor ( details ) to provide the details
of the transaction that the user is being asked to complete. This
includes the total of the order and, optionally, some line items that
can provide a detailed breakdown of what is being paid for.

Example 2 : The `details` argument 

const details = {
id : "super-store-order-123-12312" ,
displayItems : [
{
label : "Sub-total" ,
amount : { currency : "GBP" , value : "55.00" },
},
{
label : "Value-Added Tax (VAT)" ,
amount : { currency : "GBP" , value : "5.00" },
},
],
total : {
label : "Total due" ,
// The total is GBP£65.00 here because we need to 
// add shipping (below). The selected shipping 
// costs GBP£5.00. 
amount : { currency : "GBP" , value : "65.00" },
},
}; 

2.3 
Adding shipping options

Here we see an example of how to add two shipping options to the
details .

Example 3 : Adding shipping options 

const shippingOptions = [
{
id : "standard" ,
// Shipping by truck, 2 days 
label : "🚛 Envío por camión (2 dias)" ,
amount : { currency : "EUR" , value : "5.00" },
selected : true ,
},
{
id : "drone" ,
// Drone shipping, 2 hours 
label : "🚀 Drone Express (2 horas)" ,
amount : { currency : "EUR" , value : "25.00" }
},
];
Object . assign (details, { shippingOptions }); 

2.4 
Conditional modifications to payment request

Here we see how to add a processing fee for using a card on a
particular network. Notice that it requires recalculating the total.

Example 4 : Modifying payment request based on card type 

// Certain cards incur a $3.00 processing fee. 
const cardFee = {
label : "Card processing fee" ,
amount : { currency : "AUD" , value : "3.00" },
};

// Modifiers apply when the user chooses to pay with 
// a card. 
const modifiers = [
{
additionalDisplayItems : [cardFee],
supportedMethods : "https://example.com/cardpay" ,
total : {
label : "Total due" ,
amount : { currency : "AUD" , value : "68.00" },
},
data : {
supportedNetworks : networks,
},
},
];
Object . assign (details, { modifiers }); 

2.5 
Requesting specific information from the end user

Some financial transactions require a user to provide specific
information in order for a merchant to fulfill a purchase (e.g., the
user's shipping address, in case a physical good needs to be
shipped). To request this information, a merchant can pass a third
optional argument ( options ) to the
PaymentRequest constructor indicating what information they
require. When the payment request is shown, the user agent will
request this information from the end user and return it to the
merchant when the user accepts the payment request.

Example 5 : The `options` argument 

const options = {
requestPayerEmail : false ,
requestPayerName : true ,
requestPayerPhone : false ,
requestShipping : true ,
} 

2.6 
Constructing a PaymentRequest 

Having gathered all the prerequisite bits of information, we can now
construct a PaymentRequest and request that the browser present
it to the user:

Example 6 : Constructing a `PaymentRequest` 

async function doPaymentRequest ( ) {
try {
const request = new PaymentRequest (methodData, details, options);
// See below for a detailed example of handling these events 
request. onshippingaddresschange = ev => ev. updateWith (details);
request. onshippingoptionchange = ev => ev. updateWith (details);
const response = await request. show ();
await validateResponse (response);
} catch (err) {
// AbortError, SecurityError 
console . error (err);
}
}
async function validateResponse ( response ) {
try {
const errors = await checkAllValuesAreGood (response);
if (errors. length ) {
await response. retry (errors);
return validateResponse (response);
}
await response. complete ( "success" );
} catch (err) {
// Something went wrong... 
await response. complete ( "fail" );
}
}
// Must be called as a result of a click 
// or some explicit user action. 
doPaymentRequest (); 

2.7 
Handling events and updating the payment request

Prior to the user accepting to make payment, the site is given an
opportunity to update the payment request in response to user input.
This can include, for example, providing additional shipping options
(or modifying their cost), removing items that cannot ship to a
particular address, etc.

Example 7 : Registering event handlers 

const request = new PaymentRequest (methodData, details, options);
// Async update to details 
request. onshippingaddresschange = ev => {
ev. updateWith ( checkShipping (request));
};
// Sync update to the total 
request. onshippingoptionchange = ev => {
// selected shipping option 
const { shippingOption } = request;
const newTotal = {
currency : "USD" ,
label : "Total due" ,
value : calculateNewTotal (shippingOption),
};
ev. updateWith ({ total : newTotal });
};
async function checkShipping ( request ) {
try {
const { shippingAddress } = request;

await ensureCanShipTo (shippingAddress);
const { shippingOptions, total } = await calculateShipping (shippingAddress);

return { shippingOptions, total };
} catch (err) {
// Shows error to user in the payment sheet. 
return { error : `Sorry! we can't ship to your address.` };
}
} 

2.8 
Fine-grained error reporting

A developer can use the
shippingAddressErrors member of the
PaymentDetailsUpdate dictionary to indicate that there are
validation errors with specific attributes of a ContactAddress .
The shippingAddressErrors member is a
AddressErrors dictionary, whose members specifically demarcate
the fields of a physical address that are erroneous while also
providing helpful error messages to be displayed to the end user.

Example 8 

request. onshippingaddresschange = ev => {
ev. updateWith ( validateAddress (request. shippingAddress ));
};
function validateAddress ( shippingAddress ) {
const error = "Can't ship to this address." ;
const shippingAddressErrors = {
city : "FarmVille is not a real place." ,
postalCode : "Unknown postal code for your country." ,
};
// Empty shippingOptions implies that we can't ship 
// to this address. 
const shippingOptions = [];
return { error, shippingAddressErrors, shippingOptions };
} 

2.9 
POSTing payment response back to a server

It's expected that data in a PaymentResponse will be POSTed back
to a server for processing. To make this as easy as possible,
PaymentResponse can use the default toJSON steps (i.e.,
.toJSON() ) to serializes the object directly into JSON. This makes
it trivial to POST the resulting JSON back to a server using the
Fetch Standard :

Example 9 : POSTing with `fetch()` 

async function doPaymentRequest ( ) {
const payRequest = new PaymentRequest (methodData, details);
const payResponse = await payRequest. show ();
let result = "" ;
try {
const httpResponse = await fetch ( "/process-payment" , {
method : "POST" ,
headers : { "Content-Type" : "application/json" },
body : payResponse. toJSON (),
});
result = httpResponse. ok ? "success" : "fail" ;
} catch (err) {
console . error (err);
result = "fail" ;
}
await payResponse. complete (result);
}
doPaymentRequest (); 

2.10 
Using with cross-origin iframes

To indicate that a cross-origin iframe is allowed to invoke the
payment request API, the allow attribute along with the
"payment" keyword can be specified on the iframe element.

Example 10 : Using Payment Request API with cross-origin iframes 

< iframe 
src = "https://cross-origin.example" 
allow = "payment" > 
</ iframe > 

If the iframe will be navigated across multiple origins that
support the Payment Request API, then one can set allow to
"payment *" . The Permissions Policy specification provides
further details and examples.

3. 
PaymentRequest interface

WebIDL [ SecureContext , Exposed =Window ]
interface PaymentRequest : EventTarget { 
constructor ( 
sequence < PaymentMethodData > methodData , 
PaymentDetailsInit details ,
optional PaymentOptions options = {}
); 
[ NewObject ] 
Promise < PaymentResponse > show (optional Promise < PaymentDetailsUpdate > detailsPromise ); 
[ NewObject ] 
Promise < undefined > abort (); 
[ NewObject ] 
Promise < boolean > canMakePayment (); 

readonly attribute DOMString id ; 
readonly attribute ContactAddress ? shippingAddress ; 
readonly attribute DOMString ? shippingOption ; 
readonly attribute PaymentShippingType ? shippingType ; 

attribute EventHandler onshippingaddresschange ; 
attribute EventHandler onshippingoptionchange ; 
attribute EventHandler onpaymentmethodchange ; 
}; 

Note 

A developer creates a PaymentRequest to make a payment request.
This is typically associated with the user initiating a payment
process (e.g., by activating a "Buy," "Purchase," or "Checkout"
button on a web site, selecting a "Power Up" in an interactive game,
or paying at a kiosk in a parking structure). The PaymentRequest 
allows developers to exchange information with the user agent 
while the user is providing input (up to the point of user approval
or denial of the payment request).

The shippingAddress ,
shippingOption , and
shippingType attributes are populated during
processing if the requestShipping member is set.

A request 's payment-relevant browsing context is the
top-level browsing context of that PaymentRequest 's
relevant global object 's browsing context . Every
payment-relevant browsing context has a payment request is
showing boolean, which prevents showing more than one payment UI
at a time.

The payment request is showing boolean simply prevents more than
one payment UI being shown in a single browser tab. However, a
payment handler can restrict the user agent to showing
only one payment UI across all browser windows and tabs. Other payment
handlers might allow showing a payment UI across disparate browser
tabs.

3.1 
Constructor

The PaymentRequest is constructed using the supplied sequence of
PaymentMethodData methodData including any payment
method specific data , the
PaymentDetailsInit details , and the PaymentOptions 
options .

The PaymentRequest( methodData ,
details , options ) constructor MUST act as follows:

If this 's relevant global object 's associated Document is not allowed to use the "payment" 
permission, then throw a " SecurityError "
DOMException .

Establish the request's id:

If
details . id is missing, add an
id member to details and set its value
to a UUID 
[ RFC4122 ].

Let serializedMethodData be an empty list.

Process payment methods:

If the length of the methodData sequence is zero, then
throw a TypeError , optionally informing the
developer that at least one payment method is required.

Let seenPMIs be the empty set .

For each paymentMethod of methodData :

Run the
steps to validate a payment method identifier with
paymentMethod . supportedMethods . If it
returns false, then throw a RangeError exception.
Optionally, inform the developer that the payment method
identifier is invalid.

Let pmi be the result of parsing
paymentMethod . supportedMethods with
basic URL parser :

If failure, set pmi to
paymentMethod . supportedMethods .

If seenPMIs contains pmi throw a
RangeError DOMException optionally informing the
developer that this payment method identifier is a
duplicate.

Append pmi to seenPMIs .

If the data member of
paymentMethod is missing, let serializedData be null.
Otherwise, let serializedData be the result of serialize 
paymentMethod . data into a JSON
string. Rethrow any exceptions.

If serializedData is not null, and if the specification
that defines the
paymentMethod . supportedMethods 
specifies an additional data type :

Let object be the result of JSON-parsing 
serializedData .

Let idl be the result of converting object to an IDL value of the
additional data type . Rethrow any
exceptions.

Run the steps to validate payment method data ,
if any, from the specification that defines the
paymentMethod . supportedMethods 
on object . Rethrow any exceptions.

Note 

These step assures that any IDL type conversion and
validation errors are caught as early as possible.

Add the tuple
( paymentMethod . supportedMethods ,
serializedData ) to serializedMethodData .

Process the total:

Check and canonicalize total amount 
details . total . amount .
Rethrow any exceptions.

If the displayItems member of details is
present, then for each item in
details . displayItems :

Check and canonicalize amount 
item . amount . Rethrow any exceptions.

Let selectedShippingOption be null.

If the requestShipping member of options is
present and set to true, process shipping options:

Let options be an empty
sequence < PaymentShippingOption >.

If the shippingOptions member of
details is present, then:

Let seenIDs be an empty set.

For each option in
details . shippingOptions :

Check and canonicalize amount 
item . amount . Rethrow any exceptions.

If seenIDs contains
option . id , then throw a
TypeError . Optionally, inform the developer that
shipping option IDs must be unique.

Otherwise, append
option . id to seenIDs .

If option . selected is
true, then set selectedShippingOption to
option . id .

Set details . shippingOptions to
options .

Let serializedModifierData be an empty list.

Process payment details modifiers:

Let modifiers be an empty
sequence < PaymentDetailsModifier >.

If the modifiers member of details 
is present, then:

Set modifiers to
details . modifiers .

For each modifier of modifiers :

If the total member of
modifier is present, then:

Check and canonicalize total amount 
modifier . total . amount .
Rethrow any exceptions.

If the
additionalDisplayItems member
of modifier is present, then for each item of
modifier . additionalDisplayItems :

Check and canonicalize amount 
item . amount . Rethrow any
exceptions.

If the data member of
modifier is missing, let serializedData be null.
Otherwise, let serializedData be the result of
serialize 
modifier . data into a JSON
string. Rethrow any exceptions.

Add the tuple
( modifier . supportedMethods ,
serializedData ) to serializedModifierData .

Remove the data member of
modifier , if it is present.

Set details . modifiers to
modifiers .

Let request be a new PaymentRequest .

Set request . [[handler]] to null .

Set request . [[options]] to options .

Set request . [[state]] to
" created ".

Set request . [[updating]] to false.

Set request . [[details]] to details .

Set request . [[serializedModifierData]] to
serializedModifierData .

Set request . [[serializedMethodData]] to
serializedMethodData .

Set request . [[response]] to null.

Set the value of request 's shippingOption 
attribute to selectedShippingOption .

Set the value of the shippingAddress attribute
on request to null.

If options . requestShipping is set to true,
then set the value of the shippingType attribute
on request to options . shippingType . Otherwise,
set it to null.

Return request .

3.2 
id attribute

When getting, the id attribute returns this
PaymentRequest 's
[[details]] . id .

Note 

For auditing and reconciliation purposes, a merchant can associate
a unique identifier for each transaction with the
id attribute.

3.3 
show() method

Note 

The show () method is called when a developer
wants to begin user interaction for the payment request. The
show () method returns a Promise that will be
resolved when the user accepts the payment request . Some
kind of user interface will be presented to the user to facilitate
the payment request after the show () method
returns.

Each payment handler controls what happens when multiple browsing
context simultaneously call the show () method.
For instance, some payment handlers will allow multiple payment UIs
to be shown in different browser tabs/windows. Other payment
handlers might only allow a single payment UI to be shown for the
entire user agent.

The show(optional detailsPromise ) method MUST act as
follows:

Let request be this .

If the relevant global object of request does not have
transient activation , the user agent MAY :

Return a promise rejected with with a " SecurityError "
DOMException .

Note 

This allows the user agent to not require user activation, for
example to support redirect flows where a user activation may
not be present upon redirect. See 19.9 
User activation requirement for security
considerations.

See also issue
#1022 for discussion around providing more guidance in the
specification on when user agents should or should not require
a user activation for show () .

Otherwise,
consume user activation of the relevant global object .

Let document be request 's relevant global object 's
associated Document .

If document is
not fully active , then return a promise rejected
with an " AbortError " DOMException .

If document 's visibility state is not "visible" ,
then return a promise rejected with an " AbortError "
DOMException .

Optionally, if the user agent wishes to disallow the call
to show () to protect the user, then return a
promise rejected with a " SecurityError " DOMException . For
example, the user agent may limit the rate at which a page
can call show () , as described in section
19. 
Privacy and Security Considerations .

If request . [[state]] is not
" created " then return a promise rejected
with an " InvalidStateError " DOMException .

If the user agent 's payment request is showing 
boolean is true, then:

Set request . [[state]] to
" closed ".

Return a promise rejected with an " AbortError "
DOMException .

Set request . [[state]] to
" interactive ".

Let acceptPromise be a new promise .

Set request . [[acceptPromise]] to
acceptPromise .

Optionally:

Reject acceptPromise with an " AbortError "
DOMException .

Set request . [[state]] to
" closed ".

Return acceptPromise .

Note 

This allows the user agent to act as if the user had
immediately aborted the payment request , at its discretion. For example, in "private browsing"
modes or similar, user agents might take advantage of this step.

Set request 's payment-relevant browsing context 's
payment request is showing boolean to true.

Return acceptPromise and perform the remaining steps in
parallel .

Let handlers be an empty list .

For each paymentMethod tuple in
request . [[serializedMethodData]] :

Let identifier be the first element in the paymentMethod 
tuple.

Let data be the result of JSON-parsing the second element
in the paymentMethod tuple.

If the specification that defines the identifier specifies
an additional data type , then convert data to an IDL value of that type.
Otherwise, convert data to
object .

If conversion results in an exception error :

Set request . [[state]] to
" closed ".

Reject acceptPromise with error .

Set request 's payment-relevant browsing
context 's payment request is showing boolean to
false.

Terminate this algorithm.

Let registeredHandlers be a list of registered
payment handlers for the payment method identifier .
Note : Payment Handler registration 

For each handler in registeredHandlers :

Let canMakePayment be the result of running handler 's
steps to check if a payment can be made with data .

If canMakePayment is true, then append handler to
handlers .

If handlers is empty, then:

Set request . [[state]] to
" closed ".

Reject acceptPromise with " NotSupportedError "
DOMException .

Set request 's payment-relevant browsing context 's
payment request is showing boolean to false.

Terminate this algorithm.

Present a user interface that will allow the user to interact
with the handlers . The user agent SHOULD prioritize the user's
preference when presenting payment methods. The user interface
SHOULD be presented using the language and locale-based
formatting that matches the document 's document element's language , if any, or an
appropriate fallback if that is not available.

Note : Localization of the payments user interface 

If
detailsPromise was passed, then:

Run the update a PaymentRequest 's details
algorithm with detailsPromise , request , and null.

Wait for the detailsPromise to settle.
Note 

Based on how the detailsPromise settles, the update a
PaymentRequest 's details algorithm 
determines how the payment UI behaves. That is, upon
rejection of the detailsPromise , the payment request
aborts. Otherwise, upon fulfillment detailsPromise ,
the user agent re-enables the payment request UI and the
payment flow can continue.

Set request . [[handler]] be the payment
handler selected by the end-user.

Let modifiers be an empty list.

For each tuple in
[[serializedModifierData]] :

If the first element of tuple (a PMI ) matches the
payment method identifier of
request . [[handler]] , then append the second
element of tuple (the serialized method data) to modifiers .

Pass the converted second element
in the paymentMethod tuple and modifiers . Optionally, the
user agent SHOULD send the appropriate data from request to the
user-selected payment handler in order to guide the user
through the payment process. This includes the various attributes
and other internal slots of request (some MAY be excluded
for privacy reasons where appropriate).

Handling of multiple applicable modifiers in the
[[serializedModifierData]] internal slot 
is payment handler specific and beyond the scope of this
specification. Nevertheless, it is RECOMMENDED that payment
handlers use a "last one wins" approach with items in the
[[serializedModifierData]] list: that is to
say, an item at the end of the list always takes precedence over
any item at the beginning of the list (see example below).

The acceptPromise will later be resolved or rejected by the
user accepts the payment request algorithm , or the user
aborts the payment request algorithm (which are triggered
through interaction with the user interface), or the payment
handler indicates an internal error algorithm .

If document stops being fully active while the
user interface is being shown, or no longer is by the time this
step is reached, then:

Close down the user interface.

Set request 's payment-relevant browsing context 's
payment request is showing boolean to false.

3.4 
abort() method

Note 

The abort () method is called if a developer
wishes to tell the user agent to abort the payment request 
and to tear down any user interface that might be shown. The
abort () can only be called after the
show () method has been called (see
states ) and before this instance's
[[acceptPromise]] has been resolved. For
example, developers might choose to do this if the goods they are
selling are only available for a limited amount of time. If the
user does not accept the payment request within the allowed time
period, then the request will be aborted.

A user agent might not always be able to abort a request.
For example, if the user agent has delegated responsibility
for the request to another app. In this situation,
abort () will reject the returned Promise .

See also the algorithm when the user aborts the payment
request .

The abort () method MUST act as follows:

Let request be this .

If request . [[response]] is not null, and
request . [[response]] . [[retryPromise]] 
is not null, return a promise rejected with an
" InvalidStateError " DOMException .

If the value of request . [[state]] is not
" interactive " then return a promise rejected
with an " InvalidStateError " DOMException .

Let promise be a new promise .

Return promise and perform the remaining steps in
parallel .

Try to abort the current user interaction with the payment
handler and close down any remaining user interface.

Queue a task on the user interaction task source to
perform the following steps:

If it is not possible to abort the current user interaction,
then reject promise with " InvalidStateError "
DOMException and abort these steps.

Set request . [[state]] to
" closed ".

Reject the promise
request . [[acceptPromise]] with an
" AbortError " DOMException .

Resolve promise with undefined.

3.5 
canMakePayment() method

Note : canMakePayment() 

The canMakePayment () method can be used by the
developer to determine if the user agent has support for one
of the desired payment methods . See
19.8 
canMakePayment() protections .

A true result from canMakePayment () does not
imply that the user has a provisioned instrument ready for payment.

The canMakePayment () method MUST run the can
make payment algorithm .

3.6 
shippingAddress attribute

A PaymentRequest 's shippingAddress attribute
is populated when the user provides a shipping address. It is null by
default. When a user provides a shipping address, the shipping
address changed algorithm runs.

3.7 
shippingType attribute

A PaymentRequest 's shippingType attribute is
the type of shipping used to fulfill the transaction. Its value is
either a PaymentShippingType enum value, or null if none is
provided by the developer during
construction (see
PaymentOptions 's shippingType member).

3.8 
onshippingaddresschange attribute

A PaymentRequest 's onshippingaddresschange 
attribute is an EventHandler for a PaymentRequestUpdateEvent 
named shippingaddresschange .

3.9 
shippingOption attribute

A PaymentRequest 's shippingOption attribute is
populated when the user chooses a shipping option. It is null by
default. When a user chooses a shipping option, the shipping
option changed algorithm runs.

3.10 
onshippingoptionchange attribute

A PaymentRequest 's onshippingoptionchange 
attribute is an EventHandler for a PaymentRequestUpdateEvent 
named shippingoptionchange .

3.11 
onpaymentmethodchange attribute

A PaymentRequest 's onpaymentmethodchange 
attribute is an EventHandler for a PaymentMethodChangeEvent 
named " paymentmethodchange ".

3.12 
Internal Slots

Instances of PaymentRequest are created with the internal slots in the following table:

Internal Slot

Description ( non-normative )

[[serializedMethodData]] 

The methodData supplied to the constructor, but
represented as tuples containing supported methods and a string
or null for data (instead of the original object form).

[[serializedModifierData]] 

A list containing the serialized string form of each
data member for each corresponding
item in the sequence
[[details]] . modifier ,
or null if no such member was present.

[[details]] 

The current PaymentDetailsBase for the payment request
initially supplied to the constructor and then updated with calls
to updateWith () . Note that all
data members of
PaymentDetailsModifier instances contained in the
modifiers member will be removed, as they
are instead stored in serialized form in the
[[serializedModifierData]] internal slot .

[[options]] 

The PaymentOptions supplied to the constructor.

[[state]] 

The current state of the payment request, which transitions from:

" created "

The payment request is constructed and has not been presented
to the user.

" interactive "

The payment request is being presented to the user.

" closed "

The payment request completed.

The state transitions are illustrated in the
figure below:

Figure 1 
The constructor sets the initial state to
" created ". The show () 
method changes the state to
" interactive ". From there, the
abort () method or any other error can send
the state to " closed ";
similarly, the user accepts the payment request
algorithm and user aborts the payment request
algorithm will change the state to
" closed ".

[[updating]] 

True if there is a pending
updateWith () call to update the
payment request and false otherwise.

[[acceptPromise]] 

The pending Promise created during show () 
that will be resolved if the user accepts the payment request.

[[response]] 

Null, or the PaymentResponse instantiated by this
PaymentRequest .

[[handler]] 

The Payment Handler associated with this
PaymentRequest . Initialized to null .

4. 
PaymentMethodData dictionary

WebIDL dictionary PaymentMethodData { 
required DOMString supportedMethods ; 
object data ; 
}; 

A PaymentMethodData dictionary is used to indicate a set of
supported payment methods and any associated payment
method specific data for those methods.

supportedMethods member

A payment method identifier for a payment method that
the merchant web site accepts.

data member

An object that provides optional information that might be needed by
the supported payment methods. If supplied, it will be serialized .

Note 

The value of supportedMethods was changed from array to
string, but the name was left as a plural to maintain compatibility
with existing content on the Web.

5. 
PaymentCurrencyAmount dictionary

WebIDL dictionary PaymentCurrencyAmount { 
required DOMString currency ; 
required DOMString value ; 
}; 

A PaymentCurrencyAmount dictionary is used to supply monetary
amounts.

currency member

An [ ISO4217 ] well-formed 3-letter
alphabetic code (i.e., the numeric codes are not supported). Their
canonical form is upper case. However, the set of combinations of
currency code for which localized currency symbols are available is
implementation dependent.

When displaying a monetary value, it is RECOMMENDED that user
agents display the currency code, but it's OPTIONAL for user agents
to display a currency symbol. This is because currency symbols can
be ambiguous due to use across a number of different currencies
(e.g., "$" could mean any of USD, AUD, NZD, CAD, and so on.).

User agents MAY format the display of the
currency member to adhere to OS
conventions (e.g., for localization purposes).

Note : Digital currencies and ISO 4217 currency codes 

User agents implementing this specification enforce [ ISO4217 ]'s
3-letter codes format via ECMAScript’s isWellFormedCurrencyCode 
abstract operation, which is invoked as part of the check and
canonicalize amount algorithm. When a code does not adhere to
the [ ISO4217 ] defined format, a RangeError is thrown.

Current implementations will therefore allow the use of
well-formed currency codes that are not part of the official
[ ISO4217 ] list (e.g., XBT, XRP, etc.). If the provided code is
a currency that the browser knows how to display, then an
implementation will generally display the appropriate currency
symbol in the user interface (e.g., "USD" is shown as U+0024
Dollar Sign ($), "GBP" is shown as U+00A3 Pound Sign (£), "PLN"
is shown as U+007A U+0142 Złoty (zł), and the non-standard "XBT"
could be shown as U+0243 Latin Capital Letter B with Stroke (Ƀ)).

Efforts are underway at ISO to account for digital currencies,
which may result in an update to the [ ISO4217 ] registry or an
entirely new registry. The community expects this will resolve
ambiguities that have crept in through the use of non-standard
3-letter codes; for example, does "BTC" refer to Bitcoin or to a
future Bhutan currency? At the time of publication, it remains
unclear what form this evolution will take, or even the time
frame in which the work will be completed. The W3C Web Payments
Working Group is liaising with ISO so that, in the future,
revisions to this specification remain compatible with relevant
ISO registries.

value member

A valid decimal monetary value containing a monetary amount.

Example 12 : How to represent 1.234 Omani rials 

{
"currency" : "OMR" ,
"value" : "1.234" 
} 

5.1 
Validity checkers

A JavaScript string is a valid decimal monetary value 
if it consists of the following code points in the given order:

Optionally, a single U+002D (-), to indicate that the amount is
negative.

One or more code points in the range U+0030 (0) to U+0039
(9).

Optionally, a single U+002E (.) followed by one or more code points in the range U+0030 (0) to U+0039 (9).

Note 

The following regular expression is an implementation of the above
definition.
^-?[0-9]+(\.[0-9]+)?$ 

To check and canonicalize amount given a
PaymentCurrencyAmount amount , run the following steps:

If the result of IsWellFormedCurrencyCode ( amount . currency )
is false, then throw a RangeError exception, optionally informing
the developer that the currency is invalid.

If amount . value is not a valid
decimal monetary value , throw a TypeError , optionally
informing the developer that the currency is invalid.

Set amount . currency to the result of
ASCII uppercase amount . currency .

To check and canonicalize total amount given a
PaymentCurrencyAmount amount , run the
following steps:

Check and canonicalize amount amount . Rethrow any
exceptions.

If the first code point of
amount . value is U+002D (-), then throw a
TypeError optionally informing the developer that a total's value
can't be a negative number.

Note : No alteration of values 

6. 
Payment details dictionaries

6.1 
PaymentDetailsBase dictionary

WebIDL dictionary PaymentDetailsBase { 
sequence < PaymentItem > displayItems ; 
sequence < PaymentShippingOption > shippingOptions ; 
sequence < PaymentDetailsModifier > modifiers ; 
}; 

displayItems member

A sequence of PaymentItem dictionaries contains line items for
the payment request that the user agent MAY display.
Note 

shippingOptions member

A sequence containing the different shipping options for the user
to choose from.

If an item in the sequence has the
selected member set to true, then this
is the shipping option that will be used by default and
shippingOption will be set to the
id of this option without running the
shipping option changed algorithm . If more than one item
in the sequence has selected set to
true, then the user agent selects the last one in the
sequence.

The shippingOptions member is only used if
the PaymentRequest was constructed with PaymentOptions 
and requestShipping set to true.

Note 

modifiers member

A sequence of PaymentDetailsModifier dictionaries that contains
modifiers for particular payment method identifiers. For example,
it allows you to adjust the total amount based on payment method.

6.2 
PaymentDetailsInit dictionary

WebIDL dictionary PaymentDetailsInit : PaymentDetailsBase { 
DOMString id ; 
required PaymentItem total ; 
}; 

Note 

In addition to the members inherited from the PaymentDetailsBase 
dictionary, the following members are part of the
PaymentDetailsInit dictionary:

id member

A free-form identifier for this payment request.
Note 

total member

A PaymentItem containing a non-negative total amount for the
payment request.
Note 

6.3 
PaymentDetailsUpdate dictionary

WebIDL dictionary PaymentDetailsUpdate : PaymentDetailsBase { 
DOMString error ; 
PaymentItem total ; 
AddressErrors shippingAddressErrors ; 
PayerErrors payerErrors ; 
object paymentMethodErrors ; 
}; 

The PaymentDetailsUpdate dictionary is used to update the payment
request using updateWith () .

In addition to the members inherited from the PaymentDetailsBase 
dictionary, the following members are part of the
PaymentDetailsUpdate dictionary:

error member

A human-readable string that explains why goods cannot be shipped
to the chosen shipping address, or any other reason why no shipping
options are available. When the payment request is updated using
updateWith () , the
PaymentDetailsUpdate can contain a message in the
error member that will be displayed to the
user if the PaymentDetailsUpdate indicates that there are no
valid shippingOptions (and the
PaymentRequest was constructed with the
requestShipping option set to true).

total member

A PaymentItem containing a non-negative amount .
Note 

Algorithms in this specification that accept a
PaymentDetailsUpdate dictionary will throw if the
total . amount . value 
is a negative number.

shippingAddressErrors member

Represents validation errors with the shipping address that is
associated with the potential event target .

payerErrors member

Validation errors related to the payer details .

paymentMethodErrors member

Payment method specific errors.

7. 
PaymentDetailsModifier dictionary

WebIDL dictionary PaymentDetailsModifier { 
required DOMString supportedMethods ; 
PaymentItem total ; 
sequence < PaymentItem > additionalDisplayItems ; 
object data ; 
}; 

The PaymentDetailsModifier dictionary provides details that modify
the PaymentDetailsBase based on a payment method identifier .
It contains the following members:

supportedMethods member

A payment method identifier . The members of the
PaymentDetailsModifier only apply if the user selects this
payment method .

total member

A PaymentItem value that overrides the
total member in the PaymentDetailsInit 
dictionary for the payment method identifiers of the
supportedMethods member.

additionalDisplayItems member

A sequence of PaymentItem dictionaries provides additional
display items that are appended to the
displayItems member in the
PaymentDetailsBase dictionary for the payment method
identifiers in the supportedMethods 
member. This member is commonly used to add a discount or surcharge
line item indicating the reason for the different
total amount for the selected payment
method that the user agent MAY display.
Note 

It is the developer's responsibility to verify that the
total amount is the sum of the
displayItems and the
additionalDisplayItems .

data member

An object that provides optional information that might be needed by
the supported payment methods. If supplied, it will be serialized .

8. 
PaymentShippingType enum

WebIDL enum PaymentShippingType {
" shipping " ,
" delivery " ,
" pickup " 
}; 

" shipping "

This is the default and refers to the address 
being collected as the destination for shipping .

" delivery "

This refers to the address being collected as
the destination for delivery. This is commonly faster than shipping . For example, it might be used for food delivery.

" pickup "

This refers to the address being collected as
part of a service pickup. For example, this could be the address for
laundry pickup.

9. 
PaymentOptions dictionary

WebIDL dictionary PaymentOptions { 
boolean requestPayerName = false; 
boolean requestBillingAddress = false; 
boolean requestPayerEmail = false; 
boolean requestPayerPhone = false; 
boolean requestShipping = false; 
PaymentShippingType shippingType = "shipping"; 
}; 

Note 

The PaymentOptions dictionary is passed to the PaymentRequest 
constructor and provides information about the options desired for the
payment request.

requestBillingAddress member

A boolean that indicates whether the user agent SHOULD collect
and return the billing address associated with a payment
method (e.g., the billing address associated with a credit card).
Typically, the user agent will return the billing address as part of
the PaymentMethodChangeEvent 's
methodDetails . A merchant can use this
information to, for example, calculate tax in certain jurisdictions
and update the displayed total. See below for privacy considerations
regarding exposing user information .

requestPayerName member

A boolean that indicates whether the user agent SHOULD collect
and return the payer's name as part of the payment request. For
example, this would be set to true to allow a merchant to make a
booking in the payer's name.

requestPayerEmail member

A boolean that indicates whether the user agent SHOULD collect
and return the payer's email address as part of the payment request.
For example, this would be set to true to allow a merchant to email a
receipt.

requestPayerPhone member

A boolean that indicates whether the user agent SHOULD collect
and return the payer's phone number as part of the payment request.
For example, this would be set to true to allow a merchant to phone a
customer with a billing enquiry.

requestShipping member

A boolean that indicates whether the user agent SHOULD collect
and return a shipping address as part of the payment request. For
example, this would be set to true when physical goods need to be
shipped by the merchant to the user. This would be set to false for
the purchase of digital goods.

shippingType member

A PaymentShippingType enum value. Some transactions require an
address for delivery but the term "shipping"
isn't appropriate. For example, "pizza delivery" not "pizza shipping"
and "laundry pickup" not "laundry shipping". If
requestShipping is set to true, then the
shippingType member can influence the way the
user agent presents the user interface for gathering the
shipping address.

The shippingType member only affects the user
interface for the payment request.

10. 
PaymentItem dictionary

WebIDL dictionary PaymentItem { 
required DOMString label ; 
required PaymentCurrencyAmount amount ; 
boolean pending = false; 
}; 

A sequence of one or more PaymentItem dictionaries is included in
the PaymentDetailsBase dictionary to indicate what the payment
request is for and the value asked for.

label member

A human-readable description of the item. The user agent may
display this to the user.
Note : Internationalization of the label 

amount member

A PaymentCurrencyAmount containing the monetary amount for the
item.

pending member

A boolean. When set to true it means that the amount 
member is not final. This is commonly used to show items such as
shipping or tax amounts that depend upon selection of shipping
address or shipping option. User agents MAY indicate pending
fields in the user interface for the payment request.

11. 
PaymentCompleteDetails dictionary

WebIDL dictionary PaymentCompleteDetails { 
object ? data = null; 
}; 

The PaymentCompleteDetails dictionary provides additional
information from the merchant website to the payment handler when the
payment request completes.

The PaymentCompleteDetails dictionary contains the following
members:

data member

An object that provides optional information that might be needed by
the PaymentResponse associated payment method . If supplied,
it will be serialize .

12. 
PaymentComplete enum

WebIDL enum PaymentComplete {
" fail " ,
" success " ,
" unknown " 
}; 

" fail "

Indicates that processing of the payment failed. The user
agent MAY display UI indicating failure.

" success "

Indicates the payment was successfully processed. The user
agent MAY display UI indicating success.

" unknown "

The developer did not indicate success or failure and the user
agent SHOULD NOT display UI indicating success or failure.

13. 
PaymentShippingOption dictionary

WebIDL dictionary PaymentShippingOption { 
required DOMString id ; 
required DOMString label ; 
required PaymentCurrencyAmount amount ; 
boolean selected = false; 
}; 

The PaymentShippingOption dictionary has members describing a
shipping option. Developers can provide the user with one or more
shipping options by calling the
updateWith () method in response to a
change event.

id member

A string identifier used to reference this PaymentShippingOption .
It MUST be unique for a given PaymentRequest .

label member

A human-readable string description of the item. The user
agent SHOULD use this string to display the shipping option to
the user.

amount member

A PaymentCurrencyAmount containing the monetary amount for the
item.

selected member

A boolean. When true, it indicates that this is the default selected
PaymentShippingOption in a sequence. User agents SHOULD 
display this option by default in the user interface.

14. 
PaymentResponse interface

WebIDL [ SecureContext , Exposed =Window ]
interface PaymentResponse : EventTarget { 
[ Default ] object toJSON (); 

readonly attribute DOMString requestId ; 
readonly attribute DOMString methodName ; 
readonly attribute object details ; 
readonly attribute ContactAddress ? shippingAddress ; 
readonly attribute DOMString ? shippingOption ; 
readonly attribute DOMString ? payerName ; 
readonly attribute DOMString ? payerEmail ; 
readonly attribute DOMString ? payerPhone ; 

[ NewObject ] 
Promise < undefined > complete (
optional PaymentComplete result = "unknown",
optional PaymentCompleteDetails details = {}
); 
[ NewObject ] 
Promise < undefined > retry (optional PaymentValidationErrors errorFields = {}); 

attribute EventHandler onpayerdetailchange ; 
}; 

Note 

A PaymentResponse is returned when a user has selected a payment
method and approved a payment request.

14.1 
retry() method

Note 

The retry( errorFields ) method
MUST act as follows:

Let response be this .

Let request be
response . [[request]] .

Let document be request 's relevant global
object 's associated Document .

If
document is not fully active , then return a promise
rejected with an " AbortError " DOMException .

If response . [[complete]] is true, return
a promise rejected with an " InvalidStateError "
DOMException .

If response . [[retryPromise]] is not null,
return a promise rejected with an " InvalidStateError "
DOMException .

Set request . [[state]] to
" interactive ".

Let retryPromise be a new promise .

Set response . [[retryPromise]] to
retryPromise .

If errorFields was passed:

Optionally, show a warning in the developer console if any of
the following are true:

request . [[options]] . requestPayerName 
is false, and
errorFields . payer . name 
is present.

request . [[options]] . requestPayerEmail 
is false, and
errorFields . payer . email 
is present.

request . [[options]] . requestPayerPhone 
is false, and
errorFields . payer . phone 
is present.

request . [[options]] . requestShipping 
is false, and
errorFields . shippingAddress is
present.

If
errorFields . paymentMethod 
member was passed, and if required by the specification that
defines response . methodName , then
convert errorFields 's
paymentMethod member to an IDL value
of the type specified there. Otherwise, convert to object .

Set request 's payment-relevant browsing context 's
payment request is showing boolean to false.

If conversion results in a exception error :

Reject retryPromise with error .

Set user agent 's payment request is showing 
boolean to false.

Return.

By matching the members of errorFields to input fields in
the user agent's UI, indicate to the end user that something is
wrong with the data of the payment response. For example, a user
agent might draw the user's attention to the erroneous
errorFields in the browser's UI and display the value of each
field in a manner that helps the user fix each error. Similarly,
if the error member is passed,
present the error in the user agent's UI. In the case where the
value of a member is the empty string, the user agent MAY 
substitute a value with a suitable error message.

Otherwise, if errorFields was not passed, signal to the end
user to attempt to retry the payment. Re-enable any UI element that
affords the end user the ability to retry accepting the payment
request.

If
document stops being fully active while the user
interface is being shown, or no longer is by the time this step is
reached, then: 

Close down the user interface.

Set request 's payment-relevant browsing context 's
payment request is showing boolean to false.

Finally, when retryPromise settles, set
response . [[retryPromise]] to null.

Return retryPromise .
Note 

The retryPromise will later be resolved by the user accepts
the payment request algorithm , or rejected by the user
aborts the payment request algorithm , or abort the
update , or the payment handler indicates an internal error
algorithm .

14.1.1 
PaymentValidationErrors dictionary

WebIDL dictionary PaymentValidationErrors { 
PayerErrors payer ; 
AddressErrors shippingAddress ; 
DOMString error ; 
object paymentMethod ; 
}; 

payer member

Validation errors related to the payer details .

shippingAddress member

Represents validation errors with the PaymentResponse 's
shippingAddress .

error member

A general description of an error with the payment from which the
user can attempt to recover. For example, the user may recover by
retrying the payment. A developer can optionally pass the
error member on its own to give a
general overview of validation issues, or it can be passed in
combination with other members of the PaymentValidationErrors 
dictionary.
Note : Internationalization of the error 

paymentMethod member

A payment method specific errors.

14.1.2 
PayerErrors dictionary

WebIDL dictionary PayerErrors { 
DOMString email ; 
DOMString name ; 
DOMString phone ; 
}; 

The PayerErrors is used to represent validation errors with one
or more payer details .

Payer details are any of the payer's name, payer's phone
number, and payer's email.

email member

Denotes that the payer's email suffers from a validation error.
In the user agent's UI, this member corresponds to the input
field that provided the PaymentResponse 's
payerEmail attribute's value.

name member

Denotes that the payer's name suffers from a validation error. In
the user agent's UI, this member corresponds to the input field
that provided the PaymentResponse 's
payerName attribute's value.

phone member

Denotes that the payer's phone number suffers from a validation
error. In the user agent's UI, this member corresponds to the
input field that provided the PaymentResponse 's
payerPhone attribute's value.

Example 13 : Payer-related validation errors 

const payer = {
email : "The domain is invalid." ,
phone : "Unknown country code." ,
name : "Not in database." ,
};
await response. retry ({ payer }); 

14.2 
methodName attribute

The payment method identifier for the payment method 
that the user selected to fulfill the transaction.

14.3 
details attribute

An object or dictionary generated by a payment
method that a merchant can use to process or validate a
transaction (depending on the payment method ).

Note 

14.4 
shippingAddress attribute

If the requestShipping member was set to true in
the PaymentOptions passed to the PaymentRequest constructor,
then shippingAddress will be the full and final
shipping address chosen by the user.

14.5 
shippingOption attribute

If the requestShipping member was set to true in
the PaymentOptions passed to the PaymentRequest constructor,
then shippingOption will be the
id attribute of the selected shipping
option.

14.6 
payerName attribute

If the requestPayerName member was set to true in
the PaymentOptions passed to the PaymentRequest constructor,
then payerName will be the name provided by the
user.

14.7 
payerEmail attribute

If the requestPayerEmail member was set to true in
the PaymentOptions passed to the PaymentRequest constructor,
then payerEmail will be the email address chosen
by the user.

14.8 
payerPhone attribute

If the requestPayerPhone member was set to true in
the PaymentOptions passed to the PaymentRequest constructor,
then payerPhone will be the phone number chosen
by the user.

14.9 
requestId attribute

The corresponding payment request id that spawned
this payment response.

14.10 
complete() method

Note 

The complete () method is called after the user
has accepted the payment request and the
[[acceptPromise]] has been resolved. Calling the
complete () method tells the user agent 
that the payment interaction is over (and SHOULD cause any remaining
user interface to be closed).

After the payment request has been accepted and the
PaymentResponse returned to the caller, but before the caller
calls complete () , the payment request user
interface remains in a pending state. At this point the user
interface SHOULD NOT offer a cancel command because acceptance of the
payment request has been returned. However, if something goes wrong
and the developer never calls complete () then the
user interface is blocked.

For this reason, implementations MAY impose a timeout for developers
to call complete () . If the timeout expires then
the implementation will behave as if complete () 
was called with no arguments.

The complete () method MUST act as follows:

Let response be this .

If response . [[complete]] is true, return
a promise rejected with an " InvalidStateError "
DOMException .

If response . [[retryPromise]] is not null,
return a promise rejected with an " InvalidStateError "
DOMException .

Let promise be a new promise .

Let serializedData be the result of serialize 
details . data into a JSON string.

If serializing throws an exception, return a
promise rejected with that exception.

If required by the specification that defines the
response . methodName :

Let json be the result of calling JSON 's parse () 
with serializedData .

Let idl be the result of converting json to an IDL value of the type specified
by the specification that defines the
response . methodName .

If the conversion to an IDL value throws an
exception , return a promise rejected with that
exception.

If required by the specification that defines the
response . methodName , validate the members
of idl . If a member's value is invalid, return a promise
rejected with a TypeError .
Note : Opportunity to recover 

Set response . [[complete]] to true.

Return promise and perform the remaining steps in
parallel .

If document stops being fully active while the
user interface is being shown, or no longer is by the time this step
is reached, then:

Close down the user interface.

Set request 's payment-relevant browsing context 's
payment request is showing boolean to false.

Otherwise:

Close down any remaining user interface. The user
agent MAY use the value result and serializedData to
influence the user experience.

Set request 's payment-relevant browsing context 's
payment request is showing boolean to false.

Resolve promise with undefined.

14.11 
onpayerdetailchange attribute

Allows a developer to handle " payerdetailchange " events.

14.12 
Internal Slots

Instances of PaymentResponse are created with the internal slots in the following table:

Internal Slot

Description ( non-normative )

[[complete]] 

Is true if the request for payment has completed (i.e.,
complete () was called, or there was a fatal
error that made the response not longer usable), or false
otherwise.

[[request]] 

The PaymentRequest instance that instantiated this
PaymentResponse .

[[retryPromise]] 

Null, or a Promise that resolves when a user accepts the
payment request or rejects if the user aborts the payment
request .

15. 
Shipping and billing addresses

The PaymentRequest interface allows a merchant to request from the
user physical addresses for the purposes of
shipping and/or billing. A shipping address and billing
address are physical addresses .

15.1 
AddressErrors dictionary

WebIDL dictionary AddressErrors { 
DOMString addressLine ; 
DOMString city ; 
DOMString country ; 
DOMString dependentLocality ; 
DOMString organization ; 
DOMString phone ; 
DOMString postalCode ; 
DOMString recipient ; 
DOMString region ; 
DOMString sortingCode ; 
}; 

The members of the AddressErrors dictionary represent validation
errors with specific parts of a physical address . Each dictionary
member has a dual function: firstly, its presence denotes that a
particular part of an address is suffering from a validation error.
Secondly, the string value allows the developer to describe the
validation error (and possibly how the end user can fix the error).

Note 

Developers need to be aware that users might not have the ability to
fix certain parts of an address. As such, they need to be mindful not
to ask the user to fix things they might not have control over.

addressLine member

Denotes that the address line has a validation
error. In the user agent's UI, this member corresponds to the input
field that provided the ContactAddress 's
addressLine attribute's value.

city member

Denotes that the city has a validation error.
In the user agent's UI, this member corresponds to the input field
that provided the ContactAddress 's city 
attribute's value.

country member

Denotes that the country has a validation
error. In the user agent's UI, this member corresponds to the input
field that provided the ContactAddress 's
country attribute's value.

dependentLocality member

Denotes that the dependent locality has a
validation error. In the user agent's UI, this member corresponds
to the input field that provided the ContactAddress 's
dependentLocality attribute's value.

organization member

Denotes that the organization has a validation
error. In the user agent's UI, this member corresponds to the input
field that provided the ContactAddress 's
organization attribute's value.

phone member

Denotes that the phone number has a validation
error. In the user agent's UI, this member corresponds to the input
field that provided the ContactAddress 's
phone attribute's value.

postalCode member

Denotes that the postal code has a validation
error. In the user agent's UI, this member corresponds to the input
field that provided the ContactAddress 's
postalCode attribute's value.

recipient member

Denotes that the recipient has a validation
error. In the user agent's UI, this member corresponds to the input
field that provided the ContactAddress 's
addressLine attribute's value.

region member

Denotes that the region has a validation
error. In the user agent's UI, this member corresponds to the input
field that provided the ContactAddress 's
region attribute's value.

sortingCode member

The sorting code has a validation error. In
the user agent's UI, this member corresponds to the input field
that provided the ContactAddress 's
sortingCode attribute's value.

16. 
Permissions Policy integration

This specification defines a policy-controlled feature identified
by the string "payment" 
[ permissions-policy ]. Its default allowlist is 'self' .

Note 

17. 
Events

17.1 
Summary

This section is non-normative. 

Event name

Interface

Dispatched when…

Target

shippingaddresschange 

PaymentRequestUpdateEvent 

The user provides a new shipping address.

PaymentRequest 

shippingoptionchange 

PaymentRequestUpdateEvent 

The user chooses a new shipping option.

PaymentRequest 

payerdetailchange 

PaymentRequestUpdateEvent 

The user changes the payer name, the payer email, or the payer
phone (see payer detail changed algorithm ).

PaymentResponse 

paymentmethodchange 

PaymentMethodChangeEvent 

The user chooses a different payment method within a
payment handler .

PaymentRequest 

17.2 
PaymentMethodChangeEvent interface

WebIDL [ SecureContext , Exposed =Window ]
interface PaymentMethodChangeEvent : PaymentRequestUpdateEvent { 
constructor ( DOMString type , optional PaymentMethodChangeEventInit eventInitDict = {}); 
readonly attribute DOMString methodName ; 
readonly attribute object ? methodDetails ; 
}; 

17.2.1 
methodDetails attribute

When getting, returns the value it was initialized with. See
methodDetails member of
PaymentMethodChangeEventInit for more information.

17.2.2 
methodName attribute

When getting, returns the value it was initialized with. See
methodName member of
PaymentMethodChangeEventInit for more information.

17.2.3 
PaymentMethodChangeEventInit dictionary

WebIDL dictionary PaymentMethodChangeEventInit : PaymentRequestUpdateEventInit { 
DOMString methodName = ""; 
object ? methodDetails = null; 
}; 

methodName member

A string representing the payment method identifier .

methodDetails member

An object representing some data from the payment method, or
null.

17.3 
PaymentRequestUpdateEvent interface

WebIDL [ SecureContext , Exposed =Window ]
interface PaymentRequestUpdateEvent : Event { 
constructor ( DOMString type , optional PaymentRequestUpdateEventInit eventInitDict = {}); 
undefined updateWith ( Promise < PaymentDetailsUpdate > detailsPromise ); 
}; 

The PaymentRequestUpdateEvent enables developers to update the
details of the payment request in response to a user interaction.

17.3.1 
Constructor 

The PaymentRequestUpdateEvent 's
constructor ( type , eventInitDict ) MUST 
act as follows:

Let event be the result of calling
the constructor of PaymentRequestUpdateEvent with
type and eventInitDict .

Set event . [[waitForUpdate]] to
false.

Return event .

17.3.2 
updateWith() method

Note 

The updateWith () with
detailsPromise method MUST act as follows:

Let event be this .

If event 's isTrusted attribute is false, then
throw an " InvalidStateError " DOMException .

If event . [[waitForUpdate]] is
true, then throw an " InvalidStateError "
DOMException .

If event 's target is an instance of
PaymentResponse , let request be event 's
target 's [[request]] .

Otherwise, let request be the value of
event 's target .

Assert : request is an instance of PaymentRequest .

If request . [[state]] is not
" interactive ", then throw an
" InvalidStateError " DOMException .

If request . [[updating]] is true, then
throw an " InvalidStateError " DOMException .

Set event 's stop propagation flag and stop immediate propagation flag .

Set event . [[waitForUpdate]] to
true.

Let pmi be null.

If event has a methodName 
attribute, set pmi to the methodName 
attribute's value.

Run the update a PaymentRequest 's details
algorithm with detailsPromise , request , and pmi .

17.3.3 
Internal Slots

Instances of PaymentRequestUpdateEvent are created with the
internal slots in the following table:

Internal Slot

Description ( non-normative )

[[waitForUpdate]] 

A boolean indicating whether an
updateWith () -initiated update is
currently in progress.

17.3.4 
PaymentRequestUpdateEventInit dictionary

WebIDL dictionary PaymentRequestUpdateEventInit : EventInit {}; 

18. 
Algorithms

When the internal slot [[state]] of a
PaymentRequest object is set to " interactive ",
the user agent will trigger the following algorithms based on
user interaction.

18.1 
Can make payment algorithm

The can make payment algorithm checks if the user
agent supports making payment with the payment methods 
with which the PaymentRequest was constructed.

Let request be the PaymentRequest object on
which the method was called.

If request . [[state]] is not
" created ", then return a promise rejected
with an " InvalidStateError " DOMException .

Optionally, at the top-level browsing
context 's discretion, return a promise rejected with a
" NotAllowedError " DOMException .
Note 

This allows user agents to apply heuristics to detect and prevent
abuse of the calling method for fingerprinting purposes, such as
creating PaymentRequest objects with a variety of supported
payment methods and triggering the can make payment
algorithm on them one after the other. For example, a user
agent may restrict the number of successful calls that can be
made based on the top-level browsing context or the time
period in which those calls were made.

Let hasHandlerPromise be a new promise .

Return hasHandlerPromise , and perform the remaining steps in
parallel .

For each paymentMethod tuple in request .
[[serializedMethodData]] :

Let identifier be the first element in the paymentMethod 
tuple.

If the user agent has a payment handler that supports
handling payment requests for identifier , resolve
hasHandlerPromise with true and terminate this algorithm.

Resolve hasHandlerPromise with false.

18.2 
Shipping address changed algorithm

The shipping address changed algorithm runs when the user
provides a new shipping address. It MUST run the following steps:

Let request be the PaymentRequest object
that the user is interacting with.

Queue a task on the user interaction task source to
run the following steps:

Note : Privacy of recipient information 

The redactList limits the amount of personal information
about the recipient that the API shares with the merchant.

For merchants, the resulting ContactAddress object
provides enough information to, for example, calculate
shipping costs, but, in most cases, not enough information
to physically locate and uniquely identify the recipient.

Unfortunately, even with the redactList , recipient
anonymity cannot be assured. This is because in some
countries postal codes are so fine-grained that they can
uniquely identify a recipient.

Let redactList be the empty list. Set redactList to
« "organization", "phone", "recipient", "addressLine" ».

Let address be the result of running the
steps to create a contactaddress from user-provided input with redactList .

Set request . shippingAddress to
address .

Run the PaymentRequest updated algorithm with
request and " shippingaddresschange ".

18.3 
Shipping option changed algorithm

The shipping option changed algorithm runs when the user
chooses a new shipping option. It MUST run the following steps:

Let request be the PaymentRequest object
that the user is interacting with.

Queue a task on the user interaction task source to
run the following steps:

Set the shippingOption attribute on
request to the id string of the
PaymentShippingOption provided by the user.

Run the PaymentRequest updated algorithm with
request and " shippingoptionchange ".

18.4 
Payment method changed algorithm

A payment handler MAY run the payment method changed algorithm 
when the user changes payment method with methodDetails ,
which is a dictionary or an object or null, and a
methodName , which is a DOMString that represents the payment
method identifier of the payment handler the user is
interacting with.

Note : Privacy of information shared by paymentmethodchange event 

When the user selects or changes a payment method (e.g., a credit
card), the PaymentMethodChangeEvent includes redacted billing
address information for the purpose of performing tax calculations.
Redacted attributes include, but are not limited to, address line , dependent locality ,
organization , phone number ,
and recipient .

Let request be the PaymentRequest object
that the user is interacting with.

Queue a task on the user interaction task source to
run the following steps:

Assert : request . [[updating]] is false.
Only one update can take place at a time.

Assert : request . [[state]] is
" interactive ".

Fire an event named " paymentmethodchange " at
request using PaymentMethodChangeEvent , with its
methodName attribute initialized
to methodName , and its
methodDetails attribute
initialized to methodDetails .

18.5 
PaymentRequest updated algorithm

The PaymentRequest updated algorithm is run by other
algorithms above to fire an event to indicate that a user has
made a change to a PaymentRequest called request with an event
name of name :

Assert : request . [[updating]] is false. Only
one update can take place at a time.

Assert : request . [[state]] is
" interactive ".

Let event be the result of
creating an event using the PaymentRequestUpdateEvent 
interface.

Initialize event 's type attribute to name .

Dispatch event at request .

If event . [[waitForUpdate]] is
true, disable any part of the user interface that could cause another
update event to be fired.

Otherwise, set
event . [[waitForUpdate]] to true.

18.6 
Payer detail changed algorithm

The user agent MUST run the payer detail changed algorithm 
when the user changes the payer name , or the payer email , or the
payer phone in the user interface:

Let request be the PaymentRequest object
that the user is interacting with.

If request . [[response]] is null, return.

Let response be
request . [[response]] .

Queue a task on the user interaction task source to
run the following steps:

Assert : request . [[updating]] is false.

Assert : request . [[state]] is
" interactive ".

Let options be
request . [[options]] .

If payer name changed and
options . requestPayerName is true:

Set response . payerName attribute to
payer name .

If payer email changed and
options . requestPayerEmail is true:

Set response . payerEmail to payer
email .

If payer phone changed and
options . requestPayerPhone is true:

Set response . payerPhone to payer
phone .

Let event be the result of
creating an event using PaymentRequestUpdateEvent .

Initialize event 's type attribute to
" payerdetailchange ".

Dispatch event at response .

If event . [[waitForUpdate]] is
true, disable any part of the user interface that could cause
another change to the payer details to be fired.

Otherwise, set
event . [[waitForUpdate]] to true.

18.7 
User accepts the payment request algorithm

The user accepts the payment request
algorithm runs when the user accepts the payment request and
confirms that they want to pay. It MUST queue a task on the
user interaction task source to perform the following steps:

Let request be the PaymentRequest object
that the user is interacting with.

If the request . [[updating]] is true, then
terminate this algorithm and take no further action. The user
agent user interface SHOULD ensure that this never occurs.

If request . [[state]] is not
" interactive ", then terminate this algorithm and
take no further action. The user agent user interface SHOULD 
ensure that this never occurs.

If the requestShipping value of
request . [[options]] is true, then if the
shippingAddress attribute of request is null or
if the shippingOption attribute of request is
null, then terminate this algorithm and take no further action. The
user agent SHOULD ensure that this never occurs.

Let isRetry be true if
request . [[response]] is not null, false
otherwise.

Let response be
request . [[response]] if isRetry is true, or a
new PaymentResponse otherwise.

If isRetry is false, initialize the newly created response :

Set response . [[request]] to request .

Set response . [[retryPromise]] to null.

Set response . [[complete]] to false.

Set the requestId attribute value of
response to the value of
request . [[details]] . id .

Set request . [[response]] to response .

Let handler be
request . [[handler]] .

Set the methodName attribute value of
response to the payment method identifier of handler .

Set the details attribute value of response 
to an object resulting from running the handler 's steps to
respond to a payment request .

If the requestShipping value of
request . [[options]] is false, then set the
shippingAddress attribute value of response to
null. Otherwise:

Let shippingAddress be the result of
create a contactaddress from user-provided input 

Set the shippingAddress attribute value
of response to shippingAddress .

Set the shippingAddress attribute value
of request to shippingAddress .

If the requestShipping value of
request . [[options]] is true, then set the
shippingOption attribute of response to the
value of the shippingOption attribute of
request . Otherwise, set it to null.

If the requestPayerName value of
request . [[options]] is true, then set the
payerName attribute of response to the payer's
name provided by the user, or to null if none was provided.
Otherwise, set it to null.

If the requestPayerEmail value of
request . [[options]] is true, then set the
payerEmail attribute of response to the payer's
email address provided by the user, or to null if none was provided.
Otherwise, set it to null.

If the requestPayerPhone value of
request . [[options]] is true, then set the
payerPhone attribute of response to the payer's
phone number provided by the user, or to null if none was provided.
When setting the payerPhone value, the user agent
SHOULD format the phone number to adhere to [ E.164 ].

Set request . [[state]] to
" closed ".

If isRetry is true, resolve
response . [[retryPromise]] with undefined.
Otherwise, resolve request . [[acceptPromise]] 
with response .

18.8 
User aborts the payment request algorithm

The user aborts the payment request
algorithm runs when the user aborts the payment request through
the currently interactive user interface. It MUST queue a task 
on the user interaction task source to perform the following
steps:

Let request be the PaymentRequest object
that the user is interacting with.

If request . [[state]] is not
" interactive ", then terminate this algorithm and
take no further action. The user agent user interface SHOULD 
ensure that this never occurs.

Set request . [[state]] to
" closed ".

Set request 's payment-relevant browsing context 's
payment request is showing boolean to false.

Let error be an " AbortError " DOMException .

Let response be
request . [[response]] .

If response is not null:

Set response . [[complete]] to true.

Assert : response . [[retryPromise]] is
not null.

Reject response . [[retryPromise]] with
error .

Otherwise, reject request . [[acceptPromise]] 
with error .

Abort the current user interaction and close down any remaining
user interface.

18.9 
Payment handler indicates an internal error algorithm

The payment handler indicates an internal error
algorithm runs when the payment handler that the user
has selected encounters an internal error that prevents it from
completing the payment. This can occur due to reasons such as the
operating system terminating the payment handler (e.g., due to memory
pressure), or the payment handler itself encountering an
unrecoverable error.

Queue a task on the user interaction task source to
perform the following steps:

Let request be the PaymentRequest object
that the user is interacting with.

If request . [[state]] is not
" interactive ", then terminate this algorithm
and take no further action.

Let error be an " OperationError " DOMException .

Set request . [[state]] to
" closed ".

Set request 's payment-relevant browsing context 's
payment request is showing boolean to false.

Let response be
request . [[response]] .

If response is not null:

Set response . [[complete]] to true.

Assert : response . [[retryPromise]] 
is not null.

Reject response . [[retryPromise]] 
with error .

Otherwise, reject
request . [[acceptPromise]] with error .

Optionally, show a generic error message to the user
indicating that the payment could not be completed.

Optionally, log detailed error information to the developer
console for debugging purposes.

Abort the current user interaction and close down any
remaining user interface.

Note 

The " OperationError " type allows merchants to distinguish payment
handler errors from user cancellation (which uses " AbortError ").

18.10 
Update a PaymentRequest 's details algorithm

The update a PaymentRequest 's details
algorithm takes a PaymentDetailsUpdate detailsPromise , a
PaymentRequest request , and pmi that is either a DOMString or
null (a payment method identifier ). The steps are conditional
on the detailsPromise settling. If detailsPromise never settles
then the payment request is blocked. The user agent SHOULD provide
the user with a means to abort a payment request. Implementations MAY 
choose to implement a timeout for pending updates if detailsPromise 
doesn't settle in a reasonable amount of time.

In the case where a timeout occurs, or the user manually aborts, or
the payment handler decides to abort this particular payment,
the user agent MUST run the user aborts the payment request
algorithm .

Set request . [[updating]] to true.

In parallel , disable the user interface that allows the user
to accept the payment request. This is to ensure that the payment
is not accepted until the user interface is updated with any new
details.

Upon rejection of detailsPromise :

Abort the update with request and an " AbortError "
DOMException .

Upon fulfillment of detailsPromise with value value :

Let details be the result of
converting value to a
PaymentDetailsUpdate dictionary. If this throw 
an exception, abort the update with request and with the
thrown exception.

Let serializedModifierData be an empty list.

Let selectedShippingOption be null.

Let shippingOptions be an empty
sequence < PaymentShippingOption >.

Validate and canonicalize the details:

If the total member of details 
is present, then:

Check and canonicalize total amount 
details . total . amount .
If an exception is thrown, then abort the update 
with request and that exception.

If the displayItems member of
details is present, then for each item in
details . displayItems :

Check and canonicalize amount 
item . amount . If an exception is
thrown, then abort the update with request and
that exception.

If the shippingOptions member of
details is present, and
request . [[options]] . requestShipping 
is true, then:

Let seenIDs be an empty set.

For each option in
details . shippingOptions :

Check and canonicalize amount 
option . amount . If an
exception is thrown, then abort the update 
with request and that exception.

If seenIDs [ option .{{PaymentShippingOption/id}]
exists, then abort the update with request 
and a TypeError .

Append option . id to
seenIDs .

Append option to shippingOptions .

If option . selected is
true, then set selectedShippingOption to
option . id .

If the modifiers member of
details is present, then:

Let modifiers be the sequence
details . modifiers .

Let serializedModifierData be an empty list.

For each PaymentDetailsModifier modifier in
modifiers :

Run the steps to validate a payment method
identifier with
modifier . supportedMethods .
If it returns false, then abort the update 
with request and a RangeError exception.
Optionally, inform the developer that the payment
method identifier is invalid.

If the total member of
modifier is present, then:

Check and canonicalize total amount 
modifier . total . amount .
If an exception is thrown, then abort the
update with request and that exception.

If the
additionalDisplayItems 
member of modifier is present, then for each
PaymentItem item in
modifier . additionalDisplayItems :

Check and canonicalize amount 
item . amount . If an exception
is thrown, then abort the update with
request and that exception.

If the data member of
modifier is missing, let serializedData be null.
Otherwise, let serializedData be the result of
serialize 
modifier . data into a
JSON string. If it throws an exception, then abort
the update with request and that exception.

Add serializedData to serializedModifierData .

Remove the data member
of modifier , if it is present.

If the paymentMethodErrors member is
present and identifier is not null:

If required by the specification that defines the pmi ,
then convert 
paymentMethodErrors to an IDL value.

If conversion results in a exception error , 
abort the update with error .

The payment handler SHOULD display an error for
each relevant erroneous field of
paymentMethodErrors .

Update the PaymentRequest using the new details:

If the total member of details 
is present, then:

Set
request . [[details]] . total 
to details . total .

If the displayItems member of
details is present, then:

Set
request . [[details]] . displayItems 
to details . displayItems .

If the shippingOptions member of
details is present, and
request . [[options]] . requestShipping 
is true, then:

Set
request . [[details]] . shippingOptions 
to shippingOptions .

Set the value of request 's
shippingOption attribute to
selectedShippingOption .

If the modifiers member of
details is present, then:

Set
request . [[details]] . modifiers 
to details . modifiers .

Set
request . [[serializedModifierData]] 
to serializedModifierData .

If
request . [[options]] . requestShipping 
is true, and
request . [[details]] . shippingOptions 
is empty, then the developer has signified that there are
no valid shipping options for the currently-chosen
shipping address (given by request 's
shippingAddress ).

In this case, the user agent SHOULD display an error
indicating this, and MAY indicate that the
currently-chosen shipping address is invalid in some way.
The user agent SHOULD use the
error member of details , if it
is present, to give more information about why there are
no valid shipping options for that address.

Further, if
details [" shippingAddressErrors "]
member is present, the user agent SHOULD display an error
specifically for each erroneous field of the shipping
address. This is done by matching each present member of
the AddressErrors to a corresponding input field in
the shown user interface.

Similarly, if details [" payerErrors "] member is
present and request . [[options]] 's
requestPayerName ,
requestPayerEmail , or
requestPayerPhone is true, then
display an error specifically for each erroneous field.

Likewise, if
details . paymentMethodErrors is
present, then display errors specifically for each
erroneous input field for the particular payment method.

Set request . [[updating]] to false.

Update the user interface based on any changed values in
request . Re-enable user interface elements disabled prior to
running this algorithm.

18.10.1 
Abort the update

To abort the update with a
PaymentRequest request and exception exception :

Optionally, inform the developer via the console that an error
occurred while updating the payment request.

Abort the current user interaction and close down any remaining
user interface.

Queue a task on the user interaction task source to
perform the following steps:

Set request 's payment-relevant browsing context 's
payment request is showing boolean to false.

Set request . [[state]] to
" closed ".

Let response be
request . [[response]] .

If response is not null, then:

Set response . [[complete]] to
true.

Assert : response . [[retryPromise]] 
is not null.

Reject response . [[retryPromise]] 
with exception .

Otherwise, reject
request . [[acceptPromise]] with
exception .

Set request . [[updating]] to false.

Abort the algorithm.

Note 

Abort the update runs when there is a fatal error updating
the payment request, such as the supplied detailsPromise 
rejecting, or its fulfillment value containing invalid data. This
would potentially leave the payment request in an inconsistent
state since the developer hasn't successfully handled the change
event.

Consequently, the PaymentRequest moves to a
" closed " state. The error is signaled to the
developer through the rejection of the
[[acceptPromise]] , i.e., the promise returned by
show () .

Similarly, abort the update occurring during
retry () causes the
[[retryPromise]] to reject, and the
corresponding PaymentResponse 's
[[complete]] internal slot will be set to
true (i.e., it can no longer be used).

19. 
Privacy and Security Considerations

19.1 
User protections with show() method

This section is non-normative. 

To help ensure that users do not inadvertently share sensitive
credentials with an origin, this API requires that PaymentRequest's
show () method be invoked while the relevant
Window has transient activation (e.g., via a click or press).

To avoid a confusing user experience, this specification limits the
user agent to displaying one at a time via the
show () method. In addition, the user agent can
limit the rate at which a page can call show () .

19.2 
Secure contexts

This section is non-normative. 

The API defined in this specification is only exposed in a secure context - see also the Secure Contexts specification for more
details. In practice, this means that this API is only available over
HTTPS. This is to limit the possibility of payment method data (e.g.,
credit card numbers) being sent in the clear.

19.3 
Cross-origin payment requests

This section is non-normative. 

It is common for merchants and other payees to delegate checkout and
other e-commerce activities to payment service providers through an
iframe . This API supports payee-authorized cross-origin
iframes through [ HTML ]'s allow attribute.

Payment handlers have access to both the origin that hosts the
iframe and the origin of the iframe content (where the
PaymentRequest initiates).

19.4 
Encryption of data fields

This section is non-normative. 

The PaymentRequest API does not directly support encryption of
data fields. Individual payment methods may choose to include
support for encrypted data but it is not mandatory that all
payment methods support this.

19.5 
How user agents match payment handlers

This section is non-normative. 

For security reasons, a user agent can limit matching (in
show () and canMakePayment () ) to
payment handlers from the same origin as a URL payment method
identifier .

19.6 
Data usage

Payment method owners establish the privacy policies for how
user data collected for the payment method may be used. Payment
Request API sets a clear expectation that data will be used for the
purposes of completing a transaction, and user experiences associated
with this API convey that intention. It is the responsibility of the
payee to ensure that any data usage conforms to payment method
policies. For any permitted usage beyond completion of the
transaction, the payee should clearly communicate that usage to the
user.

19.7 
Exposing user information

The user agent MUST NOT share information about the user with
a developer (e.g., the shipping address ) without user consent.

In particular, the PaymentMethodData 's data 
and PaymentResponse 's details members allow
for the arbitrary exchange of data. In light of the wide range of
data models used by existing payment methods, prescribing data
specifics in this API would limit its usefulness. The
details member carries data from the payment
handler, whether Web-based (as defined by the
Web-based Payment Handler API ) or proprietary. The user agent 
MUST NOT support payment handlers unless they include adequate user
consent mechanisms (such as awareness of parties to the transaction
and mechanisms for demonstrating the intention to share data).

The user agent MUST NOT share the values of the
displayItems member or
additionalDisplayItems member for any
purpose other than to facilitate completion of the transaction.

The PaymentMethodChangeEvent enables the payee to update the
displayed total based on information specific to a selected
payment method . For example, the billing address associated
with a selected payment method might affect the tax
computation (e.g., VAT), and it is desirable that the user interface
accurately display the total before the payer completes the
transaction. At the same time, it is desirable to share as little
information as possible prior to completion of the payment.
Therefore, when a payment method defines the steps for when
a user changes payment method , it is important to minimize the
data shared via the PaymentMethodChangeEvent 's
methodDetails attribute. Requirements
and approaches for minimizing shared data are likely to vary by
payment method and might include:

Use of a " redactList " for physical addresses . The
current specification makes use of a " redactList " to redact the
address line , organization ,
phone number , and recipient 
from a shippingAddress .

Support for instructions from the payee identifying specific
elements to exclude or include from the payment method 
response data (returned through PaymentResponse . details ). The
payee might provide these instructions via
PaymentMethodData . data , enabling a payment method 
definition to evolve without requiring changes to the current API.

Where sharing of privacy-sensitive information might not be obvious
to users (e.g., when changing payment methods ), it is RECOMMENDED that user
agents inform the user of exactly what information is being shared
with a merchant.

19.8 
canMakePayment() protections

The canMakePayment () method provides feature
detection for different payment methods. It may become a
fingerprinting vector if in the future, a large number of payment
methods are available. User agents are expected to protect the user
from abuse of the method. For example, user agents can reduce user
fingerprinting by:

Rate-limiting the frequency of calls with different parameters.

For rate-limiting the user agent might look at repeated calls from:

the same registrable domain .

the top-level browsing context . Alternatively, the user agent
may block access to the API entirely for origins known to be bad
actors.

the origin of an iframe or popup window.

These rate-limiting techniques intend to increase the cost associated
with repeated calls, whether it is the cost of managing multiple
registrable domains or the user experience friction of
opening multiple windows (tabs or pop-ups).

19.9 
User activation requirement

If the user agent does not require user activation as part of the
show () method, some additional security
mitigations should be considered. Not requiring user activation
increases the risk of spam and click-jacking attacks, by allowing a
Payment Request UI to be initiated without the user interacting with
the page immediately beforehand.

In order to mitigate spam, the user agent may decide to enforce a
user activation requirement after some threshold, for example after
the user has already been shown a Payment Request UI without a user
activation on the current page. In order to mitigate click-jacking
attacks, the user agent may implement a time threshold in which
clicks are ignored immediately after a dialog is shown.

Another relevant mitigation exists in step 6 of
show () , where the document must be visible in
order to initiate the user interaction.

20. 
Accessibility Considerations

This section is non-normative. 

For the user-facing aspects of Payment Request API, implementations
integrate with platform accessibility APIs via form controls and other
input modalities. Furthermore, to increase the intelligibility of
total, shipping addresses, and contact information, implementations
format data according to system conventions.

21. 
Dependencies

This specification relies on several other underlying specifications.

ECMAScript

The term internal
slot is defined [ ECMASCRIPT ].

22. Conformance 

As well as sections marked as non-normative, all authoring guidelines, diagrams, examples, and notes in this specification are non-normative. Everything else in this specification is normative. 

The key words MAY , MUST , MUST NOT , OPTIONAL , RECOMMENDED , SHOULD , and SHOULD NOT in this document
are to be interpreted as described in
BCP 14 
[ RFC2119 ] [ RFC8174 ]
when, and only when, they appear in all
capitals, as shown here.

There is only one class of product that can claim conformance to this
specification: a user agent .

Note 

Although this specification is primarily targeted at web browsers, it
is feasible that other software could also implement this specification
in a conforming manner.

User agents MAY implement algorithms given in this specification in any
way desired, so long as the end result is indistinguishable from the
result that would be obtained by the specification's algorithms.

User agents MAY impose implementation-specific limits on otherwise
unconstrained inputs, e.g., to prevent denial of service attacks, to
guard against running out of memory, or to work around
platform-specific limitations. When an input exceeds
implementation-specific limit, the user agent MUST throw, or, in the
context of a promise, reject with, a TypeError optionally informing
the developer of how a particular input exceeded an
implementation-specific limit.

A. IDL Index 

WebIDL [ SecureContext , Exposed =Window ]
interface PaymentRequest : EventTarget { 
constructor ( 
sequence < PaymentMethodData > methodData , 
PaymentDetailsInit details ,
optional PaymentOptions options = {}
); 
[ NewObject ] 
Promise < PaymentResponse > show (optional Promise < PaymentDetailsUpdate > detailsPromise ); 
[ NewObject ] 
Promise < undefined > abort (); 
[ NewObject ] 
Promise < boolean > canMakePayment (); 

readonly attribute DOMString id ; 
readonly attribute ContactAddress ? shippingAddress ; 
readonly attribute DOMString ? shippingOption ; 
readonly attribute PaymentShippingType ? shippingType ; 

attribute EventHandler onshippingaddresschange ; 
attribute EventHandler onshippingoptionchange ; 
attribute EventHandler onpaymentmethodchange ; 
}; 

dictionary PaymentMethodData { 
required DOMString supportedMethods ; 
object data ; 
}; 

dictionary PaymentCurrencyAmount { 
required DOMString currency ; 
required DOMString value ; 
}; 

dictionary PaymentDetailsBase { 
sequence < PaymentItem > displayItems ; 
sequence < PaymentShippingOption > shippingOptions ; 
sequence < PaymentDetailsModifier > modifiers ; 
}; 

dictionary PaymentDetailsInit : PaymentDetailsBase { 
DOMString id ; 
required PaymentItem total ; 
}; 

dictionary PaymentDetailsUpdate : PaymentDetailsBase { 
DOMString error ; 
PaymentItem total ; 
AddressErrors shippingAddressErrors ; 
PayerErrors payerErrors ; 
object paymentMethodErrors ; 
}; 

dictionary PaymentDetailsModifier { 
required DOMString supportedMethods ; 
PaymentItem total ; 
sequence < PaymentItem > additionalDisplayItems ; 
object data ; 
}; 

enum PaymentShippingType {
" shipping " ,
" delivery " ,
" pickup " 
}; 

dictionary PaymentOptions { 
boolean requestPayerName = false; 
boolean requestBillingAddress = false; 
boolean requestPayerEmail = false; 
boolean requestPayerPhone = false; 
boolean requestShipping = false; 
PaymentShippingType shippingType = "shipping"; 
}; 

dictionary PaymentItem { 
required DOMString label ; 
required PaymentCurrencyAmount amount ; 
boolean pending = false; 
}; 

dictionary PaymentCompleteDetails { 
object ? data = null; 
}; 

enum PaymentComplete {
" fail " ,
" success " ,
" unknown " 
}; 

dictionary PaymentShippingOption { 
required DOMString id ; 
required DOMString label ; 
required PaymentCurrencyAmount amount ; 
boolean selected = false; 
}; 

[ SecureContext , Exposed =Window ]
interface PaymentResponse : EventTarget { 
[ Default ] object toJSON (); 

readonly attribute DOMString requestId ; 
readonly attribute DOMString methodName ; 
readonly attribute object details ; 
readonly attribute ContactAddress ? shippingAddress ; 
readonly attribute DOMString ? shippingOption ; 
readonly attribute DOMString ? payerName ; 
readonly attribute DOMString ? payerEmail ; 
readonly attribute DOMString ? payerPhone ; 

[ NewObject ] 
Promise < undefined > complete (
optional PaymentComplete result = "unknown",
optional PaymentCompleteDetails details = {}
); 
[ NewObject ] 
Promise < undefined > retry (optional PaymentValidationErrors errorFields = {}); 

attribute EventHandler onpayerdetailchange ; 
}; 

dictionary PaymentValidationErrors { 
PayerErrors payer ; 
AddressErrors shippingAddress ; 
DOMString error ; 
object paymentMethod ; 
}; 

dictionary PayerErrors { 
DOMString email ; 
DOMString name ; 
DOMString phone ; 
}; 

dictionary AddressErrors { 
DOMString addressLine ; 
DOMString city ; 
DOMString country ; 
DOMString dependentLocality ; 
DOMString organization ; 
DOMString phone ; 
DOMString postalCode ; 
DOMString recipient ; 
DOMString region ; 
DOMString sortingCode ; 
}; 

[ SecureContext , Exposed =Window ]
interface PaymentMethodChangeEvent : PaymentRequestUpdateEvent { 
constructor ( DOMString type , optional PaymentMethodChangeEventInit eventInitDict = {}); 
readonly attribute DOMString methodName ; 
readonly attribute object ? methodDetails ; 
}; 

dictionary PaymentMethodChangeEventInit : PaymentRequestUpdateEventInit { 
DOMString methodName = ""; 
object ? methodDetails = null; 
}; 

[ SecureContext , Exposed =Window ]
interface PaymentRequestUpdateEvent : Event { 
constructor ( DOMString type , optional PaymentRequestUpdateEventInit eventInitDict = {}); 
undefined updateWith ( Promise < PaymentDetailsUpdate > detailsPromise ); 
}; 

dictionary PaymentRequestUpdateEventInit : EventInit {}; 

B. 
Acknowledgements

This specification was derived from a report published previously by
the Web Platform Incubator
Community Group .

C. 
Changelog

Permalink 

Referenced in: 

§ 1. Introduction (2) (3) (4) 

§ 1.1 Goals and scope 

§ 2. Examples of usage 

§ 2.1 Declaring multiple ways of paying (2) 

§ 3.1 Constructor (2) 

§ 3.5 canMakePayment() method 

§ 4. PaymentMethodData dictionary (2) (3) 

§ 6.3 PaymentDetailsUpdate dictionary 

§ 7. PaymentDetailsModifier dictionary (2) 

§ 9. PaymentOptions dictionary 

§ 11. PaymentCompleteDetails dictionary 

§ 14.2 methodName attribute 

§ 14.3 details attribute (2) 

§ 17.1 Summary 

§ 18.1 Can make payment algorithm (2) 

§ 18.4 Payment method changed algorithm 

§ 19.4 Encryption of data fields (2) 

§ 19.6 Data usage 

§ 19.7 Exposing user information (2) (3) (4) (5) (6) 

Permalink 

Referenced in: 

§ 1.1 Goals and scope (2) (3) 

Permalink 

Referenced in: 

§ 1. Introduction 

§ 3.1 Constructor (2) 

§ 3.3 show() method 

Permalink 

Referenced in: 

§ 3.1 Constructor 

Permalink 
exported 

Referenced in: 

§ 3. PaymentRequest interface 

§ 3.3 show() method (2) (3) (4) 

§ 3.4 abort() method 

§ 3.12 Internal Slots 

§ 14.1 retry() method (2) 

§ 17.1 Summary 

§ 18.1 Can make payment algorithm 

§ 18.4 Payment method changed algorithm (2) 

§ 18.9 Payment handler indicates an internal error algorithm 

§ 18.10 Update a PaymentRequest's details algorithm (2) 

§ 19.3 Cross-origin payment requests 

§ 19.5 How user agents match payment handlers 

Permalink 

Referenced in: 

§ 3.3 show() method 

Permalink 

Referenced in: 

§ 18.7 User accepts the payment request algorithm 

Permalink 

Referenced in: 

§ 19.7 Exposing user information 

Permalink 
exported IDL 

Referenced in: 

§ 2. Examples of usage (2) 

§ 2.1 Declaring multiple ways of paying 

§ 2.2 Describing what is being paid for 

§ 2.5 Requesting specific information from the end user 

§ 2.6 Constructing a PaymentRequest 

§ 3. PaymentRequest interface (2) (3) (4) 

§ 3.1 Constructor (2) 

§ 3.2 id attribute 

§ 3.6 shippingAddress attribute 

§ 3.7 shippingType attribute 

§ 3.8 onshippingaddresschange attribute 

§ 3.9 shippingOption attribute 

§ 3.10 onshippingoptionchange attribute 

§ 3.11 onpaymentmethodchange attribute 

§ 3.12 Internal Slots (2) (3) 

§ 6.1 PaymentDetailsBase dictionary (2) 

§ 6.3 PaymentDetailsUpdate dictionary 

§ 9. PaymentOptions dictionary 

§ 13. PaymentShippingOption dictionary 

§ 14.1 retry() method 

§ 14.4 shippingAddress attribute 

§ 14.5 shippingOption attribute 

§ 14.6 payerName attribute 

§ 14.7 payerEmail attribute 

§ 14.8 payerPhone attribute 

§ 14.12 Internal Slots 

§ 15. Shipping and billing addresses 

§ 16. Permissions Policy integration (2) 

§ 17.1 Summary (2) (3) 

§ 17.3.2 updateWith() method 

§ 18. Algorithms 

§ 18.1 Can make payment algorithm (2) (3) 

§ 18.2 Shipping address changed algorithm 

§ 18.3 Shipping option changed algorithm 

§ 18.4 Payment method changed algorithm 

§ 18.5 PaymentRequest updated algorithm 

§ 18.6 Payer detail changed algorithm 

§ 18.7 User accepts the payment request algorithm 

§ 18.8 User aborts the payment request algorithm 

§ 18.9 Payment handler indicates an internal error algorithm 

§ 18.10 Update a PaymentRequest's details algorithm (2) (3) 

§ 18.10.1 Abort the update 

§ 19.3 Cross-origin payment requests 

§ 19.4 Encryption of data fields 

§ A. IDL Index 

Permalink 
exported 

Referenced in: 

Not referenced in this document. 

Permalink 

Referenced in: 

§ 3. PaymentRequest interface 

§ 3.3 show() method (2) (3) (4) 

§ 14.1 retry() method (2) 

§ 14.10 complete() method (2) 

§ 18.8 User aborts the payment request algorithm 

§ 18.9 Payment handler indicates an internal error algorithm 

§ 18.10.1 Abort the update 

Permalink 

Referenced in: 

§ 3. PaymentRequest interface 

§ 3.3 show() method (2) (3) (4) (5) 

§ 14.1 retry() method (2) (3) 

§ 14.10 complete() method (2) 

§ 18.8 User aborts the payment request algorithm 

§ 18.9 Payment handler indicates an internal error algorithm 

§ 18.10.1 Abort the update 

Permalink 

Referenced in: 

§ 3.7 shippingType attribute 

§ 6.2 PaymentDetailsInit dictionary 

Permalink 
exported IDL 

Referenced in: 

§ 3. PaymentRequest interface 

§ 3.2 id attribute 

§ 14.9 requestId attribute 

§ A. IDL Index 

Permalink 
exported IDL 

Referenced in: 

§ 2. Examples of usage (2) 

§ 3. PaymentRequest interface 

§ 3.3 show() method (2) (3) (4) (5) (6) (7) 

§ 3.4 abort() method 

§ 3.12 Internal Slots (2) 

§ 18.10 Update a PaymentRequest's details algorithm 

§ 19.1 User protections with show() method (2) (3) 

§ 19.5 How user agents match payment handlers 

§ 19.9 User activation requirement (2) 

§ A. IDL Index 

Permalink 
exported IDL 

Referenced in: 

§ 3. PaymentRequest interface 

§ 3.4 abort() method (2) (3) (4) 

§ 3.12 Internal Slots 

§ A. IDL Index 

Permalink 
exported IDL 

Referenced in: 

§ 3. PaymentRequest interface 

§ 3.5 canMakePayment() method (2) (3) 

§ 19.5 How user agents match payment handlers 

§ 19.8 canMakePayment() protections 

§ A. IDL Index 

Permalink 
exported IDL 

Referenced in: 

§ 3. PaymentRequest interface (2) 

§ 3.1 Constructor 

§ 3.6 shippingAddress attribute 

§ 14.4 shippingAddress attribute 

§ 18.2 Shipping address changed algorithm 

§ 18.7 User accepts the payment request algorithm 

§ 18.10 Update a PaymentRequest's details algorithm 

§ 19.7 Exposing user information 

§ A. IDL Index 

Permalink 
exported IDL 

Referenced in: 

§ 3. PaymentRequest interface (2) 

§ 3.1 Constructor 

§ 3.7 shippingType attribute 

§ A. IDL Index 

Permalink 
exported IDL 

Referenced in: 

§ 3. PaymentRequest interface 

§ 3.8 onshippingaddresschange attribute 

§ A. IDL Index 

Permalink 
exported IDL 

Referenced in: 

§ 3. PaymentRequest interface (2) 

§ 3.1 Constructor 

§ 3.9 shippingOption attribute 

§ 6.1 PaymentDetailsBase dictionary 

§ 14.5 shippingOption attribute 

§ 18.3 Shipping option changed algorithm 

§ 18.7 User accepts the payment request algorithm 

§ 18.10 Update a PaymentRequest's details algorithm 

§ A. IDL Index 

Permalink 
exported IDL 

Referenced in: 

§ 3. PaymentRequest interface 

§ 3.10 onshippingoptionchange attribute 

§ A. IDL Index 

Permalink 
exported IDL 

Referenced in: 

§ 3. PaymentRequest interface 

§ 3.11 onpaymentmethodchange attribute 

§ A. IDL Index 

Permalink 
exported 

Referenced in: 

§ 3.1 Constructor 

§ 3.3 show() method 

§ 18.1 Can make payment algorithm 

Permalink 
exported 

Referenced in: 

§ 3.1 Constructor 

§ 3.3 show() method (2) (3) (4) 

§ 3.12 Internal Slots 

§ 18.10 Update a PaymentRequest's details algorithm 

Permalink 
exported 

Referenced in: 

§ 3.1 Constructor 

§ 3.2 id attribute 

§ 3.12 Internal Slots 

§ 18.7 User accepts the payment request algorithm 

§ 18.10 Update a PaymentRequest's details algorithm (2) (3) (4) (5) 

Permalink 
exported 

Referenced in: 

§ 3.1 Constructor 

§ 14.1 retry() method (2) (3) (4) 

§ 18.6 Payer detail changed algorithm 

§ 18.7 User accepts the payment request algorithm (2) (3) (4) (5) (6) 

§ 18.10 Update a PaymentRequest's details algorithm (2) (3) (4) 

Permalink 

Referenced in: 

§ 3.1 Constructor 

§ 3.3 show() method (2) (3) (4) (5) (6) 

§ 3.4 abort() method (2) 

§ 14.1 retry() method 

§ 17.3.2 updateWith() method 

§ 18. Algorithms 

§ 18.1 Can make payment algorithm 

§ 18.4 Payment method changed algorithm 

§ 18.5 PaymentRequest updated algorithm 

§ 18.6 Payer detail changed algorithm 

§ 18.7 User accepts the payment request algorithm (2) 

§ 18.8 User aborts the payment request algorithm (2) 

§ 18.9 Payment handler indicates an internal error algorithm (2) 

§ 18.10.1 Abort the update 

Permalink 
exported 

Referenced in: 

§ 3.4 abort() method 

§ 3.12 Internal Slots (2) (3) (4) (5) 

Permalink 

Referenced in: 

§ 3.1 Constructor 

§ 3.3 show() method 

§ 3.12 Internal Slots 

§ 18.1 Can make payment algorithm 

Permalink 

Referenced in: 

§ 3.3 show() method 

§ 3.4 abort() method 

§ 3.12 Internal Slots 

§ 14.1 retry() method 

§ 17.3.2 updateWith() method 

§ 18. Algorithms 

§ 18.4 Payment method changed algorithm 

§ 18.5 PaymentRequest updated algorithm 

§ 18.6 Payer detail changed algorithm 

§ 18.7 User accepts the payment request algorithm 

§ 18.8 User aborts the payment request algorithm 

§ 18.9 Payment handler indicates an internal error algorithm 

Permalink 

Referenced in: 

§ 3.3 show() method (2) (3) (4) 

§ 3.4 abort() method 

§ 3.12 Internal Slots (2) 

§ 18.7 User accepts the payment request algorithm 

§ 18.8 User aborts the payment request algorithm 

§ 18.9 Payment handler indicates an internal error algorithm 

§ 18.10.1 Abort the update 

§ 18.10 Update a PaymentRequest's details algorithm 

Permalink 

Referenced in: 

§ 3.1 Constructor 

§ 17.3.2 updateWith() method 

§ 18.4 Payment method changed algorithm 

§ 18.5 PaymentRequest updated algorithm 

§ 18.6 Payer detail changed algorithm 

§ 18.7 User accepts the payment request algorithm 

§ 18.10 Update a PaymentRequest's details algorithm (2) 

§ 18.10.1 Abort the update 

Permalink 

Referenced in: 

§ 3.3 show() method 

§ 3.4 abort() method (2) 

§ 14.10 complete() method 

§ 18.7 User accepts the payment request algorithm 

§ 18.8 User aborts the payment request algorithm 

§ 18.9 Payment handler indicates an internal error algorithm 

§ 18.10.1 Abort the update 

§ 18.10 Update a PaymentRequest's details algorithm 

Permalink 

Referenced in: 

§ 3.1 Constructor 

§ 3.4 abort() method (2) 

§ 18.6 Payer detail changed algorithm (2) 

§ 18.7 User accepts the payment request algorithm (2) (3) 

§ 18.8 User aborts the payment request algorithm 

§ 18.9 Payment handler indicates an internal error algorithm 

§ 18.10.1 Abort the update 

Permalink 

Referenced in: 

§ 3.1 Constructor 

§ 3.3 show() method (2) 

§ 18.7 User accepts the payment request algorithm 

Permalink 
exported IDL 

Referenced in: 

§ 1. Introduction (2) 

§ 2. Examples of usage 

§ 2.1 Declaring multiple ways of paying 

§ 3. PaymentRequest interface 

§ 3.1 Constructor 

§ 4. PaymentMethodData dictionary (2) 

§ 19.7 Exposing user information (2) 

§ A. IDL Index (2) 

Permalink 
exported IDL 

Referenced in: 

§ 3.1 Constructor (2) (3) (4) (5) (6) 

§ 4. PaymentMethodData dictionary 

§ A. IDL Index 

Permalink 
exported IDL 

Referenced in: 

§ 1. Introduction (2) (3) 

§ 3.1 Constructor (2) (3) 

§ 4. PaymentMethodData dictionary 

§ 19.7 Exposing user information 

§ A. IDL Index 

Permalink 
exported IDL 

Referenced in: 

§ 5. PaymentCurrencyAmount dictionary (2) 

§ 5.1 Validity checkers (2) 

§ 10. PaymentItem dictionary (2) 

§ 13. PaymentShippingOption dictionary (2) 

§ A. IDL Index (2) (3) 

Permalink 
exported IDL 

Referenced in: 

§ 5. PaymentCurrencyAmount dictionary (2) 

§ 5.1 Validity checkers (2) (3) 

§ A. IDL Index 

Permalink 
exported IDL 

Referenced in: 

§ 5. PaymentCurrencyAmount dictionary 

§ 5.1 Validity checkers (2) (3) 

§ 6.2 PaymentDetailsInit dictionary 

§ 6.3 PaymentDetailsUpdate dictionary 

§ A. IDL Index 

Permalink 

Referenced in: 

§ 5. PaymentCurrencyAmount dictionary 

§ 5.1 Validity checkers 

Permalink 

Referenced in: 

§ 3.1 Constructor (2) (3) 

§ 5. PaymentCurrencyAmount dictionary 

§ 5.1 Validity checkers 

§ 18.10 Update a PaymentRequest's details algorithm (2) (3) 

Permalink 

Referenced in: 

§ 3.1 Constructor (2) 

§ 18.10 Update a PaymentRequest's details algorithm (2) 

Permalink 
exported IDL 

Referenced in: 

§ 3.12 Internal Slots 

§ 6.1 PaymentDetailsBase dictionary 

§ 6.2 PaymentDetailsInit dictionary (2) 

§ 6.3 PaymentDetailsUpdate dictionary (2) 

§ 7. PaymentDetailsModifier dictionary (2) 

§ 10. PaymentItem dictionary 

§ A. IDL Index (2) (3) 

Permalink 
exported IDL 

Referenced in: 

§ 3.1 Constructor (2) 

§ 6.1 PaymentDetailsBase dictionary 

§ 7. PaymentDetailsModifier dictionary (2) 

§ 18.10 Update a PaymentRequest's details algorithm (2) (3) (4) (5) 

§ 19.7 Exposing user information 

§ A. IDL Index 

Permalink 
exported IDL 

Referenced in: 

§ 3.1 Constructor (2) (3) 

§ 6.1 PaymentDetailsBase dictionary (2) 

§ 6.3 PaymentDetailsUpdate dictionary 

§ 18.10 Update a PaymentRequest's details algorithm (2) (3) (4) (5) 

§ A. IDL Index 

Permalink 
exported IDL 

Referenced in: 

§ 3.1 Constructor (2) (3) 

§ 3.12 Internal Slots (2) 

§ 6.1 PaymentDetailsBase dictionary 

§ 18.10 Update a PaymentRequest's details algorithm (2) (3) (4) (5) 

§ A. IDL Index 

Permalink 
exported IDL 

Referenced in: 

§ 2. Examples of usage 

§ 3. PaymentRequest interface 

§ 3.1 Constructor 

§ 6.2 PaymentDetailsInit dictionary (2) (3) (4) 

§ 7. PaymentDetailsModifier dictionary 

§ A. IDL Index (2) 

Permalink 
exported IDL 

Referenced in: 

§ 3.1 Constructor (2) 

§ 3.2 id attribute (2) 

§ 6.2 PaymentDetailsInit dictionary (2) 

§ 18.7 User accepts the payment request algorithm 

§ A. IDL Index 

Permalink 
exported IDL 

Referenced in: 

§ 3.1 Constructor 

§ 6.1 PaymentDetailsBase dictionary (2) 

§ 6.2 PaymentDetailsInit dictionary (2) 

§ 7. PaymentDetailsModifier dictionary 

§ 18.10 Update a PaymentRequest's details algorithm 

§ A. IDL Index 

Permalink 
exported IDL 

Referenced in: 

§ 2.8 Fine-grained error reporting 

§ 3. PaymentRequest interface 

§ 6.3 PaymentDetailsUpdate dictionary (2) (3) (4) (5) (6) 

§ 17.3 PaymentRequestUpdateEvent interface 

§ 17.3.2 updateWith() method 

§ 18.10 Update a PaymentRequest's details algorithm (2) 

§ A. IDL Index (2) (3) 

Permalink 
exported IDL 

Referenced in: 

§ 6.3 PaymentDetailsUpdate dictionary (2) 

§ 18.10 Update a PaymentRequest's details algorithm 

§ A. IDL Index 

Permalink 
exported IDL 

Referenced in: 

§ 6.3 PaymentDetailsUpdate dictionary (2) 

§ 18.10 Update a PaymentRequest's details algorithm (2) (3) (4) 

§ A. IDL Index 

Permalink 
exported IDL 

Referenced in: 

§ 2.8 Fine-grained error reporting (2) 

§ 6.3 PaymentDetailsUpdate dictionary 

§ 18.10 Update a PaymentRequest's details algorithm 

§ A. IDL Index 

Permalink 
exported IDL 

Referenced in: 

§ 6.3 PaymentDetailsUpdate dictionary 

§ A. IDL Index 

Permalink 
exported IDL 

Referenced in: 

§ 6.3 PaymentDetailsUpdate dictionary 

§ 18.10 Update a PaymentRequest's details algorithm (2) (3) (4) 

§ A. IDL Index 

Permalink 
exported IDL 

Referenced in: 

§ 3.1 Constructor 

§ 3.12 Internal Slots 

§ 6.1 PaymentDetailsBase dictionary (2) 

§ 7. PaymentDetailsModifier dictionary (2) (3) 

§ 18.10 Update a PaymentRequest's details algorithm 

§ A. IDL Index (2) 

Permalink 
exported IDL 

Referenced in: 

§ 3.1 Constructor 

§ 7. PaymentDetailsModifier dictionary (2) (3) 

§ 18.10 Update a PaymentRequest's details algorithm 

§ A. IDL Index 

Permalink 
exported IDL 

Referenced in: 

§ 3.1 Constructor (2) 

§ 7. PaymentDetailsModifier dictionary (2) (3) 

§ 18.10 Update a PaymentRequest's details algorithm (2) 

§ A. IDL Index 

Permalink 
exported IDL 

Referenced in: 

§ 3.1 Constructor (2) 

§ 7. PaymentDetailsModifier dictionary (2) 

§ 18.10 Update a PaymentRequest's details algorithm (2) 

§ 19.7 Exposing user information 

§ A. IDL Index 

Permalink 
exported IDL 

Referenced in: 

§ 3.1 Constructor (2) (3) 

§ 3.12 Internal Slots (2) 

§ 7. PaymentDetailsModifier dictionary 

§ 18.10 Update a PaymentRequest's details algorithm (2) (3) 

§ A. IDL Index 

Permalink 
exported IDL 

Referenced in: 

§ 3. PaymentRequest interface 

§ 3.7 shippingType attribute 

§ 8. PaymentShippingType enum 

§ 9. PaymentOptions dictionary (2) 

§ A. IDL Index (2) (3) 

Permalink 
exported IDL 

Referenced in: 

§ 8. PaymentShippingType enum 

§ A. IDL Index 

Permalink 
exported IDL 

Referenced in: 

§ 8. PaymentShippingType enum 

§ A. IDL Index 

Permalink 
exported IDL 

Referenced in: 

§ 8. PaymentShippingType enum 

§ A. IDL Index 

Permalink 
exported IDL 

Referenced in: 

§ 2. Examples of usage 

§ 3. PaymentRequest interface 

§ 3.1 Constructor 

§ 3.7 shippingType attribute 

§ 3.12 Internal Slots 

§ 6.1 PaymentDetailsBase dictionary 

§ 9. PaymentOptions dictionary (2) 

§ 14.4 shippingAddress attribute 

§ 14.5 shippingOption attribute 

§ 14.6 payerName attribute 

§ 14.7 payerEmail attribute 

§ 14.8 payerPhone attribute 

§ A. IDL Index (2) 

Permalink 
exported IDL 

Referenced in: 

§ 9. PaymentOptions dictionary 

§ A. IDL Index 

Permalink 
exported IDL 

Referenced in: 

§ 9. PaymentOptions dictionary 

§ 14.1 retry() method 

§ 14.6 payerName attribute 

§ 18.6 Payer detail changed algorithm 

§ 18.7 User accepts the payment request algorithm 

§ 18.10 Update a PaymentRequest's details algorithm 

§ A. IDL Index 

Permalink 
exported IDL 

Referenced in: 

§ 9. PaymentOptions dictionary 

§ 14.1 retry() method 

§ 14.7 payerEmail attribute 

§ 18.6 Payer detail changed algorithm 

§ 18.7 User accepts the payment request algorithm 

§ 18.10 Update a PaymentRequest's details algorithm 

§ A. IDL Index 

Permalink 
exported IDL 

Referenced in: 

§ 9. PaymentOptions dictionary 

§ 14.1 retry() method 

§ 14.8 payerPhone attribute 

§ 18.6 Payer detail changed algorithm 

§ 18.7 User accepts the payment request algorithm 

§ 18.10 Update a PaymentRequest's details algorithm 

§ A. IDL Index 

Permalink 
exported IDL 

Referenced in: 

§ 3. PaymentRequest interface 

§ 3.1 Constructor (2) 

§ 6.1 PaymentDetailsBase dictionary 

§ 6.3 PaymentDetailsUpdate dictionary 

§ 9. PaymentOptions dictionary (2) 

§ 14.1 retry() method 

§ 14.4 shippingAddress attribute 

§ 14.5 shippingOption attribute 

§ 18.7 User accepts the payment request algorithm (2) (3) 

§ 18.10 Update a PaymentRequest's details algorithm (2) (3) 

§ A. IDL Index 

Permalink 
exported IDL 

Referenced in: 

§ 3.1 Constructor 

§ 3.7 shippingType attribute 

§ 9. PaymentOptions dictionary (2) (3) 

§ A. IDL Index 

Permalink 
exported IDL 

Referenced in: 

§ 6.1 PaymentDetailsBase dictionary (2) 

§ 6.2 PaymentDetailsInit dictionary (2) 

§ 6.3 PaymentDetailsUpdate dictionary (2) 

§ 7. PaymentDetailsModifier dictionary (2) (3) (4) 

§ 10. PaymentItem dictionary (2) 

§ 18.10 Update a PaymentRequest's details algorithm 

§ A. IDL Index (2) (3) (4) (5) (6) 

Permalink 
exported IDL 

Referenced in: 

§ 10. PaymentItem dictionary (2) 

§ A. IDL Index 

Permalink 
exported IDL 

Referenced in: 

§ 3.1 Constructor (2) (3) (4) (5) 

§ 6.2 PaymentDetailsInit dictionary 

§ 6.3 PaymentDetailsUpdate dictionary (2) 

§ 10. PaymentItem dictionary (2) 

§ 18.10 Update a PaymentRequest's details algorithm (2) (3) (4) 

§ A. IDL Index 

Permalink 
exported IDL 

Referenced in: 

§ 10. PaymentItem dictionary 

§ A. IDL Index 

Permalink 
exported IDL 

Referenced in: 

§ 11. PaymentCompleteDetails dictionary (2) (3) 

§ 14. PaymentResponse interface 

§ A. IDL Index (2) 

Permalink 
exported IDL 

Referenced in: 

§ 11. PaymentCompleteDetails dictionary 

§ 14.10 complete() method 

§ A. IDL Index 

Permalink 
exported IDL 

Referenced in: 

§ 12. PaymentComplete enum 

§ 14. PaymentResponse interface 

§ A. IDL Index (2) 

Permalink 
exported IDL 

Referenced in: 

§ 12. PaymentComplete enum 

§ A. IDL Index 

Permalink 
exported IDL 

Referenced in: 

§ 12. PaymentComplete enum 

§ A. IDL Index 

Permalink 
exported IDL 

Referenced in: 

§ 12. PaymentComplete enum 

§ A. IDL Index 

Permalink 
exported IDL 

Referenced in: 

§ 3.1 Constructor 

§ 6.1 PaymentDetailsBase dictionary 

§ 13. PaymentShippingOption dictionary (2) (3) (4) 

§ 18.3 Shipping option changed algorithm 

§ 18.10 Update a PaymentRequest's details algorithm 

§ A. IDL Index (2) 

Permalink 
exported IDL 

Referenced in: 

§ 3.1 Constructor (2) (3) 

§ 6.1 PaymentDetailsBase dictionary 

§ 13. PaymentShippingOption dictionary 

§ 14.5 shippingOption attribute 

§ 18.10 Update a PaymentRequest's details algorithm (2) 

§ A. IDL Index 

Permalink 
exported IDL 

Referenced in: 

§ 13. PaymentShippingOption dictionary 

§ A. IDL Index 

Permalink 
exported IDL 

Referenced in: 

§ 13. PaymentShippingOption dictionary 

§ 18.10 Update a PaymentRequest's details algorithm 

§ A. IDL Index 

Permalink 
exported IDL 

Referenced in: 

§ 3.1 Constructor 

§ 6.1 PaymentDetailsBase dictionary (2) (3) 

§ 13. PaymentShippingOption dictionary 

§ 18.10 Update a PaymentRequest's details algorithm 

§ A. IDL Index 

Permalink 
exported IDL 

Referenced in: 

§ 2. Examples of usage 

§ 2.9 POSTing payment response back to a server (2) 

§ 3. PaymentRequest interface 

§ 3.12 Internal Slots 

§ 11. PaymentCompleteDetails dictionary 

§ 14. PaymentResponse interface (2) 

§ 14.1 retry() method (2) (3) (4) (5) 

§ 14.1.1 PaymentValidationErrors dictionary 

§ 14.1.2 PayerErrors dictionary (2) (3) 

§ 14.10 complete() method 

§ 14.12 Internal Slots (2) 

§ 17.1 Summary 

§ 17.3.2 updateWith() method 

§ 18.7 User accepts the payment request algorithm 

§ 18.10 Update a PaymentRequest's details algorithm 

§ 19.7 Exposing user information (2) 

§ A. IDL Index (2) 

Permalink 
exported IDL 

Referenced in: 

§ 14. PaymentResponse interface 

§ 14.1 retry() method (2) 

§ 18.10 Update a PaymentRequest's details algorithm 

§ A. IDL Index 

Permalink 
exported IDL 

Referenced in: 

§ 14. PaymentResponse interface 

§ 14.1 retry() method (2) 

§ 14.1.1 PaymentValidationErrors dictionary (2) 

§ A. IDL Index (2) 

Permalink 
exported IDL 

Referenced in: 

§ 14.1 retry() method (2) (3) 

§ 14.1.1 PaymentValidationErrors dictionary 

§ A. IDL Index 

Permalink 
exported IDL 

Referenced in: 

§ 14.1 retry() method 

§ 14.1.1 PaymentValidationErrors dictionary 

§ A. IDL Index 

Permalink 
exported IDL 

Referenced in: 

§ 14.1 retry() method 

§ 14.1.1 PaymentValidationErrors dictionary (2) (3) 

§ A. IDL Index 

Permalink 
exported IDL 

Referenced in: 

§ 14.1 retry() method (2) 

§ 14.1.1 PaymentValidationErrors dictionary 

§ A. IDL Index 

Permalink 
exported IDL 

Referenced in: 

§ 6.3 PaymentDetailsUpdate dictionary 

§ 14.1.1 PaymentValidationErrors dictionary 

§ 14.1.2 PayerErrors dictionary (2) 

§ 18.10 Update a PaymentRequest's details algorithm 

§ A. IDL Index (2) (3) 

Permalink 

Referenced in: 

§ 6.3 PaymentDetailsUpdate dictionary 

§ 14.1.1 PaymentValidationErrors dictionary 

§ 14.1.2 PayerErrors dictionary 

Permalink 
exported IDL 

Referenced in: 

§ 14.1 retry() method 

§ 14.1.2 PayerErrors dictionary 

§ A. IDL Index 

Permalink 
exported IDL 

Referenced in: 

§ 14.1 retry() method 

§ 14.1.2 PayerErrors dictionary 

§ A. IDL Index 

Permalink 
exported IDL 

Referenced in: 

§ 14.1 retry() method 

§ 14.1.2 PayerErrors dictionary 

§ A. IDL Index 

Permalink 
exported IDL 

Referenced in: 

§ 14. PaymentResponse interface 

§ 14.1 retry() method 

§ 14.10 complete() method (2) (3) 

§ 18.7 User accepts the payment request algorithm 

§ A. IDL Index 

Permalink 
exported IDL 

Referenced in: 

§ 14. PaymentResponse interface 

§ 18.7 User accepts the payment request algorithm 

§ 19.7 Exposing user information (2) 

§ A. IDL Index 

Permalink 
exported IDL 

Referenced in: 

§ 14. PaymentResponse interface 

§ 14.1.1 PaymentValidationErrors dictionary 

§ 18.7 User accepts the payment request algorithm (2) (3) 

§ A. IDL Index 

Permalink 
exported IDL 

Referenced in: 

§ 14. PaymentResponse interface 

§ 18.7 User accepts the payment request algorithm (2) 

§ A. IDL Index 

Permalink 
exported IDL 

Referenced in: 

§ 14. PaymentResponse interface 

§ 14.1.2 PayerErrors dictionary 

§ 14.6 payerName attribute 

§ 18.6 Payer detail changed algorithm 

§ 18.7 User accepts the payment request algorithm 

§ A. IDL Index 

Permalink 
exported IDL 

Referenced in: 

§ 14. PaymentResponse interface 

§ 14.1.2 PayerErrors dictionary 

§ 14.7 payerEmail attribute 

§ 18.6 Payer detail changed algorithm 

§ 18.7 User accepts the payment request algorithm 

§ A. IDL Index 

Permalink 
exported IDL 

Referenced in: 

§ 14. PaymentResponse interface 

§ 14.1.2 PayerErrors dictionary 

§ 14.8 payerPhone attribute 

§ 18.6 Payer detail changed algorithm 

§ 18.7 User accepts the payment request algorithm (2) 

§ A. IDL Index 

Permalink 
exported IDL 

Referenced in: 

§ 14. PaymentResponse interface 

§ 18.7 User accepts the payment request algorithm 

§ A. IDL Index 

Permalink 
exported IDL 

Referenced in: 

§ 14. PaymentResponse interface 

§ 14.10 complete() method (2) (3) (4) (5) (6) (7) 

§ 14.12 Internal Slots 

§ A. IDL Index 

Permalink 
exported IDL 

Referenced in: 

§ 14. PaymentResponse interface 

§ A. IDL Index 

Permalink 

Referenced in: 

§ 14.1 retry() method 

§ 14.10 complete() method (2) 

§ 18.7 User accepts the payment request algorithm 

§ 18.8 User aborts the payment request algorithm 

§ 18.9 Payment handler indicates an internal error algorithm 

§ 18.10.1 Abort the update 

§ 18.10 Update a PaymentRequest's details algorithm 

Permalink 

Referenced in: 

§ 14.1 retry() method 

§ 17.3.2 updateWith() method 

§ 18.7 User accepts the payment request algorithm 

Permalink 

Referenced in: 

§ 3.4 abort() method 

§ 14.1 retry() method (2) (3) (4) 

§ 14.10 complete() method 

§ 18.7 User accepts the payment request algorithm (2) 

§ 18.8 User aborts the payment request algorithm (2) 

§ 18.9 Payment handler indicates an internal error algorithm (2) 

§ 18.10.1 Abort the update (2) 

§ 18.10 Update a PaymentRequest's details algorithm 

Permalink 

Referenced in: 

§ 8. PaymentShippingType enum (2) 

§ 9. PaymentOptions dictionary 

§ 14.4 shippingAddress attribute 

§ 19.7 Exposing user information 

Permalink 

Referenced in: 

§ 9. PaymentOptions dictionary 

Permalink 
exported IDL 

Referenced in: 

§ 2.8 Fine-grained error reporting 

§ 6.3 PaymentDetailsUpdate dictionary 

§ 14.1.1 PaymentValidationErrors dictionary 

§ 15.1 AddressErrors dictionary (2) 

§ 18.10 Update a PaymentRequest's details algorithm 

§ A. IDL Index (2) (3) 

Permalink 
exported IDL 

Referenced in: 

§ 15.1 AddressErrors dictionary 

§ A. IDL Index 

Permalink 
exported IDL 

Referenced in: 

§ 15.1 AddressErrors dictionary 

§ A. IDL Index 

Permalink 
exported IDL 

Referenced in: 

§ 15.1 AddressErrors dictionary 

§ A. IDL Index 

Permalink 
exported IDL 

Referenced in: 

§ 15.1 AddressErrors dictionary 

§ A. IDL Index 

Permalink 
exported IDL 

Referenced in: 

§ 15.1 AddressErrors dictionary 

§ A. IDL Index 

Permalink 
exported IDL 

Referenced in: 

§ 15.1 AddressErrors dictionary 

§ A. IDL Index 

Permalink 
exported IDL 

Referenced in: 

§ 15.1 AddressErrors dictionary 

§ A. IDL Index 

Permalink 
exported IDL 

Referenced in: 

§ 15.1 AddressErrors dictionary 

§ A. IDL Index 

Permalink 
exported IDL 

Referenced in: 

§ 15.1 AddressErrors dictionary 

§ A. IDL Index 

Permalink 
exported IDL 

Referenced in: 

§ 15.1 AddressErrors dictionary 

§ A. IDL Index 

Permalink 

Referenced in: 

§ 3.1 Constructor 

Permalink 

Referenced in: 

§ 3.8 onshippingaddresschange attribute 

§ 18.2 Shipping address changed algorithm 

Permalink 

Referenced in: 

§ 3.10 onshippingoptionchange attribute 

§ 6.1 PaymentDetailsBase dictionary 

§ 18.3 Shipping option changed algorithm 

Permalink 

Referenced in: 

§ 14.11 onpayerdetailchange attribute 

§ 18.6 Payer detail changed algorithm 

Permalink 

Referenced in: 

§ 3.11 onpaymentmethodchange attribute 

§ 18.4 Payment method changed algorithm 

Permalink 
exported IDL 

Referenced in: 

§ 3.11 onpaymentmethodchange attribute 

§ 9. PaymentOptions dictionary 

§ 17.1 Summary 

§ 17.2 PaymentMethodChangeEvent interface 

§ 18.4 Payment method changed algorithm (2) 

§ 19.7 Exposing user information (2) 

§ A. IDL Index 

Permalink 
exported 

Referenced in: 

Not referenced in this document. 

Permalink 
exported IDL 

Referenced in: 

§ 9. PaymentOptions dictionary 

§ 17.2 PaymentMethodChangeEvent interface 

§ 18.4 Payment method changed algorithm 

§ 19.7 Exposing user information 

§ A. IDL Index 

Permalink 
exported IDL 

Referenced in: 

§ 17.2 PaymentMethodChangeEvent interface 

§ 17.3.2 updateWith() method (2) 

§ 18.4 Payment method changed algorithm 

§ A. IDL Index 

Permalink 
exported IDL 

Referenced in: 

§ 17.2 PaymentMethodChangeEvent interface 

§ 17.2.1 methodDetails attribute 

§ 17.2.2 methodName attribute 

§ 17.2.3 PaymentMethodChangeEventInit dictionary 

§ A. IDL Index (2) 

Permalink 
exported IDL 

Referenced in: 

§ 17.2.2 methodName attribute 

§ 17.2.3 PaymentMethodChangeEventInit dictionary 

§ A. IDL Index 

Permalink 
exported IDL 

Referenced in: 

§ 17.2.1 methodDetails attribute 

§ 17.2.3 PaymentMethodChangeEventInit dictionary 

§ A. IDL Index 

Permalink 
exported IDL 

Referenced in: 

§ 3.8 onshippingaddresschange attribute 

§ 3.10 onshippingoptionchange attribute 

§ 17.1 Summary (2) (3) 

§ 17.2 PaymentMethodChangeEvent interface 

§ 17.3 PaymentRequestUpdateEvent interface (2) 

§ 17.3.1 Constructor (2) 

§ 17.3.2 updateWith() method 

§ 17.3.3 Internal Slots 

§ 18.5 PaymentRequest updated algorithm 

§ 18.6 Payer detail changed algorithm 

§ A. IDL Index (2) 

Permalink 
exported IDL 

Referenced in: 

§ 17.3 PaymentRequestUpdateEvent interface 

§ 17.3.1 Constructor 

§ A. IDL Index 

Permalink 
exported IDL 

Referenced in: 

§ 3.12 Internal Slots (2) 

§ 6.3 PaymentDetailsUpdate dictionary (2) 

§ 13. PaymentShippingOption dictionary 

§ 17.3 PaymentRequestUpdateEvent interface 

§ 17.3.2 updateWith() method (2) (3) 

§ 17.3.3 Internal Slots 

§ A. IDL Index 

Permalink 

Referenced in: 

§ 17.3.1 Constructor 

§ 17.3.2 updateWith() method (2) (3) 

§ 18.5 PaymentRequest updated algorithm (2) 

§ 18.6 Payer detail changed algorithm (2) 

Permalink 
exported IDL 

Referenced in: 

§ 17.2.3 PaymentMethodChangeEventInit dictionary 

§ 17.3 PaymentRequestUpdateEvent interface 

§ 17.3.4 PaymentRequestUpdateEventInit dictionary 

§ A. IDL Index (2) (3) 

Permalink 

Referenced in: 

§ 3.5 canMakePayment() method 

§ 18.1 Can make payment algorithm 

Permalink 

Referenced in: 

§ 3.6 shippingAddress attribute 

Permalink 

Referenced in: 

§ 3.9 shippingOption attribute 

§ 6.1 PaymentDetailsBase dictionary 

Permalink 
exported 

Referenced in: 

§ 19.7 Exposing user information 

Permalink 

Referenced in: 

§ 18.2 Shipping address changed algorithm 

§ 18.3 Shipping option changed algorithm 

Permalink 

Referenced in: 

§ 17.1 Summary 

Permalink 
exported 

Referenced in: 

§ 3.3 show() method (2) 

§ 3.12 Internal Slots 

§ 14.1 retry() method 

§ 14.12 Internal Slots 

Permalink 
exported 

Referenced in: 

§ 3.3 show() method (2) 

§ 3.4 abort() method 

§ 3.12 Internal Slots 

§ 14.1 retry() method 

§ 14.12 Internal Slots 

§ 18.10 Update a PaymentRequest's details algorithm 

Permalink 
exported 

Referenced in: 

§ 3.3 show() method 

§ 14.1 retry() method 

Permalink 

Referenced in: 

§ 3.3 show() method (2) 

§ 17.3.2 updateWith() method 

Permalink 
exported 

Referenced in: 

§ 14.1 retry() method 

§ 18.10 Update a PaymentRequest's details algorithm (2) (3) (4) (5) (6) (7) (8) (9) (10) (11) (12) (13) 

Permalink 

Referenced in: 

§ 1. Introduction 

§ 3. PaymentRequest interface (2) 

§ 3.3 show() method (2) (3) (4) 

§ 3.4 abort() method (2) (3) 

§ 3.5 canMakePayment() method 

§ 6.1 PaymentDetailsBase dictionary (2) 

§ 6.2 PaymentDetailsInit dictionary 

§ 9. PaymentOptions dictionary (2) (3) (4) (5) (6) 

§ 10. PaymentItem dictionary (2) 

§ 12. PaymentComplete enum (2) (3) 

§ 13. PaymentShippingOption dictionary (2) 

§ 14.1 retry() method 

§ 14.10 complete() method (2) 

§ 17.3.2 updateWith() method 

§ 18. Algorithms 

§ 18.1 Can make payment algorithm 

§ 18.7 User accepts the payment request algorithm (2) (3) 

§ 18.8 User aborts the payment request algorithm 

§ 19.7 Exposing user information (2) (3) 

Changes from between CR2 until now:

Allow payment handlers to report back errors via Payment Request API … 
[Editorial] Update spec to reference web-based-payment-handler ( #1054 ) 
Update README.md ( #1047 ) 
[Spec] Restore changes since REC ( #1029 ) 
Status section improvement; tidy ( #1026 ) 
[Spec] Relax user activation requirement for show() ( #1009 ) 
[Spec] Only allow show() to be called in a foreground tab ( #1005 ) 
[Spec] Fix broken reference to permissions-policy ( #1007 ) 
remove note since in practice we have a 1.1 branch ( #1001 ) 
Define concepts for converting and validating `.data` ( #977 ) 
passing data on complete() ( #982 ) 
Validate .data on construction ( #976 ) 
Add internationalization support for human readable labels ( #971 ) 
Don't set state to closed when transient acttivation rejects 
Reject with SecurityError when no transient activation ( #961 ) 
Drop PaymentAddress, shipping + billing address support ( #955 ) 
Recommend Payment UI matches doc's language ( #944 ) 
Drop metion of ¤ 
Added based on issue 936 ( #937 ) 
Consume user activation ( #916 ) 
Remove hasEnrolledInstrument() ( #930 ) 
Remove merchant validation ( #929 ) 
Deprecate allowpaymentrequest attribute ( #928 ) 
Remove recommendation to localize sheet based on body element ( #896 ) 
Remove requirement to reject after document is inactive ( #875 ) 
Add PaymentRequest.prototype.hasEnrolledInstrument() ( #833 ) 
[Editorial] Fix exporting of three algorithms ( #1061 ) 

Changes from between CR1 and CR2:

Changes stemming from privacy review: ( #856 ) 
Set [[waitForUpdate]] to true on dispatch of payerdetailchange ( #857 ) 
Add privacy protection to MerchantValidationEvent's validationURL ( #850 ) 
Describe privacy inplications of changing payment method ( #849 ) 
Redact dependentLocality from shippingAddress ( #846 ) 
Changes resulting from 28 February PING privacy review ( #843 ) 
Do .data IDL conversion in constructor ( #829 ) 
Integrate with Feature Policy ( #822 ) 
Remove regionCode attribute ( #823 ) 
Clarify retry() when errorFields are not passed ( #825 ) 
Attach 'payment request is showing boolean' to top-level browsing con… 
Clarify when the user can abort the payment request algorithm ( #810 ) 
Warn when errorFields don't match request[[options]] ( #807 ) 
Support requesting billing address ( #749 ) 
Added section and paragraph on accessibility considerations ( #802 ) 
Change what canMakePayment() means ( #806 ) 
Remove PaymentAddress' languageCode ( #765 ) 
Remove PaymentItem.type ( #794 ) 
Rename PayerErrorFields to PayerErrors ( #789 ) 
Add paymentMethodErrors, payerErrors, to PaymentDetailsUpdate ( #768 ) 
Add MerchantValidationEvent.prototype.methodName ( #776 ) 
Add support for merchant validation ( #751 ) 
Support fine-grained errors for payment methods ( #752 ) 
Add error member to PaymentValidationErrors ( #747 ) 
Check doc fully active during response.complete() ( #748 ) 
Drop prefixes, suffixes from error field members ( #745 ) 
Added information about redactList to privacy consideration ( #738 ) 
Add PaymentResponse.prototype.onpayerdetailchange ( #724 ) 
retry() interacting with abort the update ( #723 ) 
teach retry() about payerErrors ( #721 ) 
Define PaymentResponse.prototype.retry() method ( #720 ) 
add PaymentMethodChangeEvent event ( #695 ) 
Add fine-grained errors reporting for PaymentAddress ( #712 ) 
add PaymentAddress.regionCode attribute ( #690 ) 
remove currencySystem member ( #694 ) 
add redactList for PaymentAddress ( #654 ) 
show() must be triggered by user activation 
Feat: allow show() to take optional detailsPromise 
Feat: adds PaymentItemType enum + PaymentItem.type ( #666 ) 
Add localization hint for payment sheet ( #656 ) 
Return event, because useful 
privacy: dont share line items ( #670 ) 
Assure PaymentRequest.id is a UUID (closes #588) 

D. References 

D.1 Normative references 

[contact-picker] 
Contact Picker API . Peter Beverloo. W3C. 8 July 2024. W3C Working Draft. URL: https://www.w3.org/TR/contact-picker/ 
[dom] 
DOM Standard . Anne van Kesteren. WHATWG. Living Standard. URL: https://dom.spec.whatwg.org/ 
[E.164] 
The international public telecommunication numbering plan . ITU-T. November 2010. Recommendation. URL: https://www.itu.int/rec/dologin_pub.asp?lang=e&id=T-REC-E.164-201011-I!!PDF-E&type=items 
[ecma-402] 
ECMAScript Internationalization API Specification . Ecma International. URL: https://tc39.es/ecma402/ 
[ECMASCRIPT] 
ECMAScript Language Specification . Ecma International. URL: https://tc39.es/ecma262/multipage/ 
[fetch] 
Fetch Standard . Anne van Kesteren. WHATWG. Living Standard. URL: https://fetch.spec.whatwg.org/ 
[HTML] 
HTML Standard . Anne van Kesteren; Domenic Denicola; Dominic Farolino; Ian Hickson; Philip Jägenstedt; Simon Pieters. WHATWG. Living Standard. URL: https://html.spec.whatwg.org/multipage/ 
[infra] 
Infra Standard . Anne van Kesteren; Domenic Denicola. WHATWG. Living Standard. URL: https://infra.spec.whatwg.org/ 
[ISO4217] 
Currency codes - ISO 4217 . ISO. 2015. International Standard. URL: http://www.iso.org/iso/home/standards/currency_codes.htm 
[payment-method-id] 
Payment Method Identifiers . Marcos Caceres. W3C. 8 September 2022. W3C Recommendation. URL: https://www.w3.org/TR/payment-method-id/ 
[permissions-policy] 
Permissions Policy . Ian Clelland. W3C. 6 October 2025. W3C Working Draft. URL: https://www.w3.org/TR/permissions-policy-1/ 
[RFC2119] 
Key words for use in RFCs to Indicate Requirement Levels . S. Bradner. IETF. March 1997. Best Current Practice. URL: https://www.rfc-editor.org/rfc/rfc2119 
[RFC4122] 
A Universally Unique IDentifier (UUID) URN Namespace . P. Leach; M. Mealling; R. Salz. IETF. July 2005. Proposed Standard. URL: https://www.rfc-editor.org/rfc/rfc4122 
[RFC8174] 
Ambiguity of Uppercase vs Lowercase in RFC 2119 Key Words . B. Leiba. IETF. May 2017. Best Current Practice. URL: https://www.rfc-editor.org/rfc/rfc8174 
[url] 
URL Standard . Anne van Kesteren. WHATWG. Living Standard. URL: https://url.spec.whatwg.org/ 
[web-based-payment-handler] 
Web-based Payment Handler API . Ian Jacobs; Stephen McGruer; Jinho Bang. W3C. 18 February 2026. W3C Working Draft. URL: https://www.w3.org/TR/web-based-payment-handler/ 
[WEBIDL] 
Web IDL Standard . Edgar Chen; Timothy Gu. WHATWG. Living Standard. URL: https://webidl.spec.whatwg.org/ 

D.2 Informative references 

[rfc6454] 
The Web Origin Concept . A. Barth. IETF. December 2011. Proposed Standard. URL: https://www.rfc-editor.org/rfc/rfc6454 
[secure-contexts] 
Secure Contexts . Mike West. W3C. 10 November 2023. CRD. URL: https://www.w3.org/TR/secure-contexts/ 

↑