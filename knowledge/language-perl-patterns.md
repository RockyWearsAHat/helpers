# Perl Programming Patterns: Regex, Text Processing, and Ecosystem

## Overview

Perl remains unmatched for **text processing and regular expressions**. Its culture emphasizes **many ways to do it** (TIMTOWTDI); flexibility is both strength and weakness. Modern Perl (5.36+) adopted strict by default and brought signatures, post-deref syntax, and `try/catch`. Use cases: sysadmin scripting, bioinformatics (sequence processing), text mining, one-liners. Trade-offs: flexible syntax permits unreadable code; weaker than Python on general-purpose programming; declining but loyal community.

## Regular Expressions: Perl's Crown Jewel

### Syntax Essentials

Perl regex is **NFA backtracking engine** (similar to Python, JavaScript, Java). Matches are greedy by default.

```perl
my $text = "hello world";

# Match (returns true/false and captures)
if ($text =~ /(\w+)\s+(\w+)/) {
  say "Captured: $1, $2";  # $1 = "hello", $2 = "world"
}

# Substitution
$text =~ s/world/perl/;  # Replace first match
$text =~ s/o/0/g;         # Replace all (g = global)

# Split
my @words = split /\s+/, $text;
```

### Quantifiers and Greediness

- `*` (0+), `+` (1+), `?` (0-1), `{n,m}` (between n and m) — **greedy** by default
- `*?`, `+?`, `??`, `{n,m}?` — **non-greedy** (lazy)

```perl
my $html = "<b>hello</b> <b>world</b>";
my @greedy = $html =~ /<b>(.*?)<\/b>/g;    # ("hello", "world") — non-greedy
my @wrong = $html =~ /<b>(.*)<\/b>/g;      # ("hello</b> <b>world") — greedy eats too much
```

### Character Classes and Modifiers

```perl
/\d+/          # digits
/\w+/          # word chars ([a-zA-Z0-9_])
/\s+/          # whitespace
/[a-z]+/       # custom range
/[^abc]/       # negation (not a, b, c)
/colou?r/      # optional: "color" or "colour"
/hello|world/  # alternation (left-to-right, first match wins)
```

Modifiers:

```perl
/pattern/i     # case-insensitive
/pattern/x     # extended (ignore whitespace, allow comments)
/pattern/s     # single-line (. matches \n)
/pattern/m     # multi-line (^ $ match line boundaries, not just string)
```

### Lookahead and Lookbehind (Non-Capturing Assertions)

```perl
# Lookahead: (?=...) positive, (?!...) negative
/\d+(?=px)/          # digits followed by "px" (doesn't consume "px")

# Lookbehind: (?<=...) positive, (?<!...) negative
/(?<=\$)\d+/         # digits preceded by "$"

# Example: extract price without $
my $price = $amount =~ /(?<=\$)([\d.]+)/;
```

### Modifiers: Named Captures

Modern Perl (5.10+) supports named captures:

```perl
if ($text =~ /(?<first>\w+)\s+(?<second>\w+)/) {
  say $+{first};   # "hello"
  say $+{second};  # "world"
}
```

Preferred over `$1, $2` for readability.

### One-Liners with Regex

Perl shines in shell one-liners:

```bash
# Print lines matching pattern
perl -ne 'print if /pattern/' file.txt

# Count matches
perl -ne 'push @m, $1 while /(\w+)/g; END { say scalar @m }' file.txt

# Replace without modifying original file
perl -i.bak -pe 's/old/new/g' file.txt

# Extract field (like awk)
perl -lane 'print $F[2]' file.txt  # $F = split fields, -a = autosplit

# Transpose CSV
perl -F, -lane 'push @{$col[$_]}, $F[$_] for 0..$#F; END { print "@$_" for @col }' file.csv
```

## File Handling and Text Processing

### Reading Files

```perl
# Line by line
open my $fh, "<", "file.txt" or die "Cannot open: $!";
while (my $line = <$fh>) {
  chomp($line);  # remove trailing newline
  say $line;
}
close $fh;

# Modern: slurp entire file
use File::Slurp;
my $content = read_file("file.txt");
my @lines = read_file("file.txt", chomp => 1);

# Perl one-liner equivalent
perl -e 'local $/; my $content = <STDIN>' < file.txt
```

### File I/O Patterns

```perl
# Read JSON file
use JSON;
my $data = decode_json(scalar read_file("data.json"));

# Process CSV
use Text::CSV;
open my $fh, "<", "data.csv" or die $!;
my $csv = Text::CSV->new();
while (my $row = $csv->getline($fh)) {
  say "Name: ", $row->[0];
}

# Write file with error handling
open my $out, ">", "output.txt" or die "Cannot write: $!";
print $out "Hello, $_\n" for @lines;
close $out or die "Error closing file: $!";
```

### Text Transformation

```perl
# Case conversion
my $upper = uc $text;
my $lower = lc $text;
my $title = ucfirst $text;

# String reversal
my $reversed = reverse($text);  # string reversal
my @reversed = reverse(@array);  # array reversal

# Split and join
my @words = split /\s+/, $text;
my $joined = join("-", @words);

# String functions
my $length = length($text);
my $substr = substr($text, 0, 5);
my $index = index($text, "pattern");
my $replaced = $text =~ s/old/new/r;  # non-destructive substitution (/r flag)
```

## Modules and CPAN

### Installing Modules

CPAN (Comprehensive Perl Archive Network) is the standard library repository.

```bash
# Via cpan CLI
cpan install JSON File::Slurp Path::Tiny

# Via cpanm (faster, recommended)
cpanm JSON File::Slurp Path::Tiny

# Check installed
perl -e 'use JSON; print "installed\n"'
```

### Essential Modules

**Data & Serialization**:

- `JSON` / `JSON::XS` — JSON encoding/decoding
- `YAML` — YAML parsing
- `Data::Dumper` — debug data structures

**File & System**:

- `File::Slurp` — read/write files simply
- `Path::Tiny` — modern path handling
- `File::Glob` — globbing

**Text & Regex**:

- `Text::CSV` — CSV parsing
- `Regexp::Common` — pre-built regexes
- `String::Similarity` — string distance

**Web & Networking**:

- `LWP::UserAgent` — HTTP client
- `Mojolicious` — web framework
- `Dancer2` — lightweight web framework

**Database**:

- `DBI` — database abstraction
- `DBD::mysql`, `DBD::Pg` — database drivers

**Testing**:

- `Test::More` — unit testing
- `Test::MockObject` — mocking

## Object-Oriented Perl

### Moose: Full-Featured OOP

Moose provides declarative OOP with type checking, roles, metaprogramming.

```perl
package Person;
use Moose;

has name => (is => 'ro', isa => 'Str');    # read-only attribute
has age  => (is => 'rw', isa => 'Int');    # read-write
has roles => (is => 'ro', isa => 'ArrayRef[Str]', default => sub { [] });

sub greet {
  my $self = shift;
  say "Hello, I am " . $self->name;
}

__PACKAGE__->meta->make_immutable;

package main;
my $person = Person->new(name => "Alice", age => 30);
$person->greet();
```

**Strengths**: declarative, strong type checking, roles (mixins), extensible.

**Weaknesses**: runtime overhead; slower startup than base Perl.

### Moo: Lightweight Alternative

Minimal OOP, compile-time optimization, Moose-compatible.

```perl
package Person;
use Moo;

has name => (is => 'ro');
has age  => (is => 'rw');

sub greet { say "Hello, I am $_[0]->name" }

1;
```

Preferred for performance-critical scripts; Moose for large applications.

### bless: Base OOP

Classic Perl object system (manual, but lightweight):

```perl
package Counter;

sub new {
  my $class = shift;
  return bless { count => 0 }, $class;
}

sub increment {
  my $self = shift;
  $self->{count}++;
}

package main;
my $counter = Counter->new();
$counter->increment();
say $counter->{count};
```

Rarely used in modern code; Moo/Moose preferred.

## Perl 5 vs. Raku (formerly Perl 6)

| Aspect | Perl 5 | Raku |
|--------|--------|------|
| **Year** | 1994 (ongoing) | 2015 |
| **Compatibility** | Stable, backwards-compatible | New language, not compatible |
| **Culture** | TIMTOWTDI, implicit | Consistency, explicit |
| **Regex** | NFA, PCRE-like | Rule-based, grammar-oriented |
| **Typing** | Dynamic, optional gradual | Static (optional), gradual |
| **Concurrency** | Threads (heavy), async (experimental) | Async/await, lightweight threads |
| **Adoption** | Established in sysadmin, bioinformatics | Niche research, education |

Raku is a different language despite the name. Choose Perl 5 for existing codebases and production scripts; Raku for new projects prioritizing consistency and modern features, if adoption is acceptable.

## Use Cases and Domains

### Sysadmin Scripting

Perl's regex and text processing dominate system administration:

```perl
# Parse Apache logs and count IPs
perl -pale '$seen{$F[0]}++; END { print "$_ => $seen{$_}\n" for keys %seen }' access.log
```

### Bioinformatics

Text processing for DNA/protein sequences:

```perl
use Bio::Seq;
my $seq = Bio::Seq->new(-seq => "ATCGATCG", -alphabet => "dna");
say "Reverse complement: ", $seq->revcom()->seq();
```

### Web Frameworks

Dancer2 and Mojolicious enable web development:

```perl
use Dancer2;

get '/' => sub {
  return { message => "Hello, Perl Web!" };
};

start;
```

### Data Processing Pipelines

ETL (extract, transform, load) workflows:

```perl
# Read CSV, transform, output JSON
my $csv = Text::CSV->new();
my @objects;
while (my $row = $csv->getline($fh)) {
  push @objects, { id => $row->[0], name => $row->[1] };
}
say encode_json(\@objects);
```

## Decline and Longevity

Perl's adoption declined as Python rose (more readable, ML-friendly) and Node.js took web development. However, Perl remains deeply embedded in:

- Large Unix/Linux infrastructures (legacy sysadmin code)
- Bioinformatics (strong institutional presence, BioPerl ecosystem)
- Regular expression puzzles and code golf
- Niche text-processing roles

Perl 5 is stable and well-maintained; likely to remain so. New projects increasingly choose Python or Node; Perl projects tend to be long-lived and resistant to replacement.

**Summary**: Modern Perl is powerful for text processing and regex, with modern syntax (strict, signatures) and strong ecosystem (CPAN, Moose). Adoption is low but loyal; suitable for specialized scripts and domains where regex dominance matters.