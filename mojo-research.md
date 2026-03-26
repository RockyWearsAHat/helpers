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

# Untitled
Source: https://www.youtube.com/channel/UCnyNcN11T3ZqnN8xVJ-JKQA

About Press Copyright Contact us Creators Advertise Developers Terms Privacy Policy & Safety How YouTube works Test new features NFL Sunday Ticket &copy; 2026 Google LLC