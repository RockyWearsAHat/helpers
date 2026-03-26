# Gleam programming language
Source: https://gleam.run/

The power of a type system, the expressiveness of functional
programming, and the reliability of the highly concurrent, fault
tolerant Erlang runtime, with a familiar and modern syntax. 

import gleam/io 

pub fn main () {
io . println ( "hello, friend!" )
} 

Kindly supported by 

and sponsors like you! 

Reliable and scalable 
Running on the battle-tested Erlang virtual machine that powers
planet-scale systems such as WhatsApp and Ericsson, Gleam is ready for
workloads of any size. 
Thanks to its multi-core actor based concurrency system that can run
millions of concurrent green threads, fast immutable data
structures, and a concurrent garbage collector that never stops
the world, your service can scale and stay lightning fast with ease. 

pub fn main () {
let subject = process . new_subject ()

// Spawn a child green thread 
process . spawn ( fn () {
// Send a message back to the parent 
process . send (subject, "Hello, Joe!" )
})

// Wait for the message to arrive 
echo process . receive (subject, 100 )
}

Ready when you are 
Gleam comes with compiler, build tool, formatter, editor integrations,
and package manager all built in, so creating a Gleam project is just
running gleam new 
As part of the wider BEAM ecosystem, Gleam programs can use thousands of
published packages, whether they are written in Gleam, Erlang, or
Elixir. 

➜ (main) gleam add gleam_json
Resolving versions
Downloading packages
Downloaded 2 packages in 0.01s
Added gleam_json v0.5.0
➜ (main) gleam test
Compiling thoas
Compiling gleam_json
Compiling app
Compiled in 1.67s
Running app_test.main
.
1 tests, 0 failures 

Here to help 
No null values, no exceptions, clear error messages, and a practical
type system. Whether you're writing new code or maintaining old code,
Gleam is designed to make your job as fun and stress-free as possible. 

error: Unknown record field

┌─ ./src/app.gleam:8:16
│
8 │ user.alias
│ ^^^^^^ Did you mean `name`? 

The value being accessed has this type:
User

It has these fields:
.name

Multilingual 
Gleam makes it easy to use code written in other BEAM languages such as
Erlang and Elixir, so there's a rich ecosystem of thousands of open
source libraries for Gleam users to make use of. 
Gleam can additionally compile to JavaScript, enabling you to use your
code in the browser, or anywhere else JavaScript can run. It also
generates TypeScript definitions, so you can interact with your Gleam
code confidently, even from the outside. 

@external (erlang, "Elixir.HPAX" , "new" )
pub fn new (size: Int ) -> Table 

pub fn register_event_handler () {
let el = document . query_selector ( "a" )
element . add_event_listener (el, fn () {
io . println ( "Clicked!" )
})
} 

Friendly 💜 
As a community, we want to be friendly too. People from around the
world, of all backgrounds, genders, and experience levels are welcome
and respected equally. See our community code of conduct for more. 
Black lives matter. Trans rights are human rights. No nazi bullsh*t.

Lovely people 
If you enjoy Gleam consider becoming a sponsor (or tell your boss to) 

You're still here? 
Well, that's all this page has to say. Maybe you should go read the language tour! 
Let's go! Wanna keep in touch? 
Subscribe to the Gleam newsletter 
We send emails at most a few times a year, and we'll never share your
email with anyone else. 
This site is protected by reCAPTCHA and the Google Privacy Policy and Terms of Service apply.

---

# GitHub - gleam-lang/gleam: ⭐️ A friendly language for building type-safe, scalable systems! · GitHub
Source: https://github.com/gleam-lang/gleam

Gleam is a friendly language for building type-safe systems that scale! For more
information see the website . 

Support Gleam! 

Gleam is not owned by a corporation, instead it is kindly supported by its
sponsors. If you like Gleam please consider sponsoring the project or members
of the core team . 

Thank you so much! 💖

---

# Welcome to the Gleam language tour! 💫 - The Gleam Language Tour
Source: https://tour.gleam.run/

Welcome to the Gleam language tour! 💫 

This tour covers all aspects of the Gleam language, and assuming you have some
prior programming experience should teach you everything you need to write
real programs in Gleam.

The tour is interactive! The code shown is editable and will be compiled and
evaluated as you type. Anything you print using 

io.println 

or echo will be shown in the bottom section, along with any compile
errors and warnings. 
To evaluate Gleam code the tour compiles Gleam to JavaScript and runs it, 
all entirely within your browser window.

If at any point you get stuck or have a question do not hesitate to ask in
the Gleam Discord server . We're here
to help, and if you find something confusing then it's likely others will too,
and we want to know about it so we can improve the tour.

OK, let's go. Click "Next" to get started, click "Contents" to jump to a
specific topic, or go here to read everything in
one page.