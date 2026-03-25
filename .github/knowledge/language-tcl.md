# Tcl Best Practices

## Tcl Philosophy

Tcl (Tool Command Language) is an embeddable scripting language. Its unique design — everything is a string, everything is a command — makes it extremely flexible for configuration, testing, and automation. Paired with Tk, it provides cross-platform GUI development.

- **Everything is a string**: All values, including code, are strings. Commands interpret strings as needed.
- **Everything is a command**: `if`, `while`, `proc` — all are commands, not syntax.
- **Embeddable**: Designed to be embedded in C/C++ applications as a scripting layer.

## Core Syntax

```tcl
# Variables
set name "Alice"
set age 30

# String interpolation (in double quotes)
puts "Hello, $name! Age: $age"

# No interpolation (in braces)
puts {Literal $name — not expanded}

# Math expressions
set result [expr {$age + 10}]

# Lists (space-separated values)
set fruits {apple banana cherry}
lappend fruits "date"
set second [lindex $fruits 1]      ;# banana
set count [llength $fruits]         ;# 4

# Dictionaries
set user [dict create name Alice age 30 email alice@test.com]
dict set user score 95
set name [dict get $user name]
dict for {key value} $user {
    puts "$key = $value"
}

# Arrays (associative)
set config(host) "localhost"
set config(port) 8080
parray config
```

## Procedures

```tcl
proc greet {name {greeting "Hello"}} {
    return "$greeting, $name!"
}

puts [greet "Alice"]            ;# Hello, Alice!
puts [greet "Bob" "Hi"]         ;# Hi, Bob!

# Variable arguments
proc sum {args} {
    set total 0
    foreach n $args {
        set total [expr {$total + $n}]
    }
    return $total
}

puts [sum 1 2 3 4 5]  ;# 15

# Upvar (access caller's variables)
proc increment {varName {amount 1}} {
    upvar 1 $varName var
    set var [expr {$var + $amount}]
}

set count 0
increment count
increment count 5
puts $count  ;# 6
```

## Control Flow

```tcl
# if/elseif/else
if {$age >= 18} {
    puts "Adult"
} elseif {$age >= 13} {
    puts "Teenager"
} else {
    puts "Child"
}

# switch
switch -exact -- $status {
    "active"   { puts "Active user" }
    "pending"  { puts "Pending approval" }
    "disabled" { puts "Account disabled" }
    default    { puts "Unknown status: $status" }
}

# foreach
foreach fruit $fruits {
    puts "Fruit: $fruit"
}

# foreach with multiple variables
foreach {key value} [dict get $config] {
    puts "$key -> $value"
}

# while
set i 0
while {$i < 10} {
    puts $i
    incr i
}

# for
for {set i 0} {$i < 10} {incr i} {
    puts $i
}
```

## String Operations

```tcl
# String commands
string length $name          ;# 5
string toupper $name         ;# ALICE
string range $name 0 2       ;# Ali
string match {A*} $name      ;# 1 (glob match)
string first "li" $name      ;# 1 (index)

# Regular expressions
if {[regexp {^\d{4}-\d{2}-\d{2}$} $date]} {
    puts "Valid date format"
}

# Capture groups
regexp {(\w+)@(\w+\.\w+)} $email -> user domain
puts "User: $user, Domain: $domain"

# String substitution
regsub -all {\s+} $text " " cleaned

# Format
set msg [format "%-10s %5d %8.2f" $name $age $score]
```

## Namespaces and Packages

```tcl
# Namespace
namespace eval ::myapp {
    variable version "1.0"

    proc initialize {} {
        variable version
        puts "Starting myapp v$version"
    }

    proc ::myapp::helper {args} {
        # internal helper
    }
}

::myapp::initialize

# Package
package provide mypackage 1.0

namespace eval ::mypackage {
    namespace export public_proc

    proc public_proc {args} {
        puts "Public: $args"
    }
}

# Using a package
package require mypackage 1.0
::mypackage::public_proc "hello"
```

## Error Handling

```tcl
# try/on/trap (Tcl 8.6+)
try {
    set data [read_file $path]
    set config [parse_json $data]
} on error {msg opts} {
    puts stderr "Error: $msg"
    # dict get $opts -errorinfo  ;# stack trace
} finally {
    cleanup
}

# catch (classic)
if {[catch {expr {1 / 0}} result opts]} {
    puts "Error: $result"
} else {
    puts "Result: $result"
}

# throw
proc validate_port {port} {
    if {$port < 1 || $port > 65535} {
        throw {VALIDATION PORT} "Invalid port: $port"
    }
}
```

## Tk GUI

```tcl
package require Tk

# Simple GUI
wm title . "My Application"

ttk::label .name_label -text "Name:"
ttk::entry .name_entry -textvariable name
ttk::button .greet_btn -text "Greet" -command {
    tk_messageBox -message "Hello, $name!"
}

grid .name_label  -row 0 -column 0 -padx 5 -pady 5
grid .name_entry  -row 0 -column 1 -padx 5 -pady 5
grid .greet_btn   -row 1 -column 0 -columnspan 2 -pady 10
```

## Key Rules

1. **Always brace expressions.** `expr {$a + $b}` not `expr $a + $b` — braces prevent double substitution and enable byte-compilation.
2. **Use `dict` over arrays** for structured data. Dicts are values (passable, returnable); arrays are not.
3. **Brace code bodies.** `if {cond} {body}` — braces prevent premature substitution.
4. **Use namespaces.** Avoid polluting the global namespace with procedures.
5. **Use Tcl 8.6+ features**: `try/on/finally`, coroutines, `lmap`, tailcall.
6. **Security: never `eval` user input.** Use `{*}` (argument expansion) instead of string-building commands.

---

*Sources: Tcl Developer Xchange (tcl-lang.org), Practical Programming in Tcl and Tk (Welch), Tcl/Tk 8.6 Manual*
