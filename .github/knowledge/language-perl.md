# Perl Best Practices

## Modern Perl Philosophy

Modern Perl (5.36+) has strict mode, signatures, and post-deref syntax. Perl remains unmatched for text processing, regex, and one-liners.

- **TIMTOWTDI**: "There Is More Than One Way To Do It" — Perl's motto. But choose the readable way.
- **Use strict and warnings**: Always. No exceptions.
- **CPAN is a superpower**: 200,000+ modules. Use proven libraries.

## Setup

```perl
# Modern Perl header (5.36+ enables strict, warnings, say, signatures)
use v5.36;

# Pre-5.36 equivalent
use strict;
use warnings;
use feature qw(say signatures);
no warnings 'experimental::signatures';
```

## Regex Mastery

```perl
# Match and capture
my $text = "Date: 2024-03-15";
if ($text =~ /(\d{4})-(\d{2})-(\d{2})/) {
    my ($year, $month, $day) = ($1, $2, $3);
    say "$month/$day/$year";
}

# Named captures
if ($text =~ /(?<year>\d{4})-(?<month>\d{2})-(?<day>\d{2})/) {
    say $+{year};
}

# Substitution
(my $clean = $input) =~ s/\s+/ /g;

# Non-destructive substitution (5.14+)
my $clean = $input =~ s/\s+/ /gr;

# Extended regex with comments
my $email_re = qr{
    (?<user>   [a-zA-Z0-9._%+-]+ )
    \@
    (?<domain> [a-zA-Z0-9.-]+ \. [a-zA-Z]{2,} )
}x;
```

## Data Structures

```perl
# Arrays
my @numbers = (1, 2, 3, 4, 5);
my @slice = @numbers[1..3];  # (2, 3, 4)
push @numbers, 6;
my $last = pop @numbers;

# Hashes
my %user = (
    name  => "Alice",
    age   => 30,
    email => 'alice@test.com',
);
my @keys = keys %user;
my @values = values %user;

# References (for nested structures)
my $aref = [1, 2, 3];            # arrayref
my $href = {name => "Bob"};       # hashref
my @matrix = ([1,2,3], [4,5,6]); # array of arrayrefs

# Dereferencing
my @arr = @{$aref};       # full deref
my $val = $aref->[0];     # element access
my @arr = $aref->@*;      # postfix deref (5.24+)
```

## Subroutines

```perl
# Modern signatures (5.36+)
sub greet($name, $greeting = "Hello") {
    say "$greeting, $name!";
}

# Classic
sub greet_classic {
    my ($name, $greeting) = @_;
    $greeting //= "Hello";  # defined-or
    say "$greeting, $name!";
}

# Return values
sub divmod($a, $b) {
    return (int($a / $b), $a % $b);
}
my ($q, $r) = divmod(17, 5);
```

## Object-Oriented (Moo/Moose)

```perl
# Moo (lightweight, production standard)
package User;
use Moo;
use Types::Standard qw(Str Int);

has name  => (is => 'ro', isa => Str, required => 1);
has age   => (is => 'rw', isa => Int);
has email => (is => 'ro', isa => Str);

sub greet($self) {
    say "Hi, I'm " . $self->name;
}

# Roles (Perl's answer to interfaces/traits)
package Printable;
use Moo::Role;

requires 'to_string';

sub print_self($self) {
    say $self->to_string;
}
```

## File and Text Processing

```perl
# Read entire file
use File::Slurper 'read_text';
my $content = read_text('file.txt');

# Line-by-line processing
open my $fh, '<', 'data.txt' or die "Cannot open: $!";
while (my $line = <$fh>) {
    chomp $line;
    next if $line =~ /^\s*#/;  # skip comments
    # process $line
}
close $fh;

# One-liner paradigm: read → process → print
perl -lane 'print $F[2] if $F[0] eq "ERROR"' logfile.txt

# Common one-liner flags:
# -n  loop over lines (no print)
# -p  loop over lines (auto-print)
# -l  chomp input, add newline on output
# -a  autosplit into @F
# -e  inline code
# -i  in-place edit
```

## Error Handling

```perl
# eval/die is Perl's try/catch
eval {
    dangerous_operation();
    1;
} or do {
    my $err = $@;
    warn "Operation failed: $err";
};

# Better: use Try::Tiny
use Try::Tiny;
try {
    dangerous_operation();
} catch {
    warn "Caught error: $_";
} finally {
    cleanup();
};

# die with references for structured errors
die { code => 404, message => "Not found" };
```

## Key Rules

1. **Always `use strict; use warnings;`** — or `use v5.36;` which enables both.
2. **Use lexical filehandles** (`open my $fh, ...`) — never bareword handles.
3. **Check return values** of `open`, `close`, system calls. Use `autodie` to auto-check.
4. **Use `//` (defined-or)** instead of `||` when 0 or "" are valid values.
5. **Prefer CPAN modules** over reinventing: Path::Tiny, JSON::MaybeXS, HTTP::Tiny, DBI.
6. **Avoid bareword strings, global variables, and two-arg open**.

---

_Sources: Modern Perl (chromatic), perldoc.perl.org, Perl Best Practices (Conway), CPAN documentation_
