# Ada Conventions and Idioms

## Ada Philosophy

Ada is a statically typed, compiled language designed for safety-critical and real-time systems — avionics, defense, rail, medical devices, and space. It prioritizes correctness, readability, and long-term maintainability over developer convenience.

- **Correctness by construction**: The type system catches errors at compile time that other languages catch at runtime (or not at all).
- **Readability**: Code is read far more than written. Ada's verbose syntax is intentional — it reads like English.
- **Strong typing with range constraints**: Types carry domain constraints the compiler enforces.

## Type System

```ada
-- Scalar types with constraints
type Temperature is digits 6 range -273.15 .. 1_000_000.0;
type Percentage  is range 0 .. 100;
type Latitude    is digits 8 range -90.0 .. 90.0;
type Longitude   is digits 8 range -180.0 .. 180.0;

-- Enumeration types
type Day is (Monday, Tuesday, Wednesday, Thursday, Friday, Saturday, Sunday);
type Traffic_Light is (Red, Yellow, Green);

-- Derived types (incompatible by design)
type Meters  is new Float;
type Seconds is new Float;
type Meters_Per_Second is new Float;

-- Cannot accidentally add Meters + Seconds — compile error!
-- Must explicitly convert: Meters_Per_Second(Distance) / Meters_Per_Second(Time)

-- Subtypes (compatible but constrained)
subtype Weekday is Day range Monday .. Friday;
subtype Positive_Temperature is Temperature range 0.0 .. Temperature'Last;

-- Modular types (unsigned with wraparound)
type Byte is mod 256;  -- 0..255, wraps on overflow
```

## Records and Access Types

```ada
-- Record types (structs)
type Date is record
   Year  : Integer range 1900 .. 2100;
   Month : Integer range 1 .. 12;
   Day   : Integer range 1 .. 31;
end record;

type Employee is record
   Name       : String (1 .. 50);
   Birth_Date : Date;
   Salary     : Float range 0.0 .. Float'Last;
   Department : String (1 .. 20);
end record;

-- Discriminated records (variant records)
type Shape_Kind is (Circle, Rectangle, Triangle);

type Shape (Kind : Shape_Kind) is record
   X, Y : Float;  -- common fields
   case Kind is
      when Circle    => Radius : Float;
      when Rectangle => Width, Height : Float;
      when Triangle  => A, B, C : Float;
   end case;
end record;

-- Usage
My_Circle : Shape := (Kind => Circle, X => 0.0, Y => 0.0, Radius => 5.0);
```

## Packages (Modules)

```ada
-- Specification (interface) — geometry.ads
package Geometry is

   type Point is record
      X, Y : Float;
   end record;

   function Distance (P1, P2 : Point) return Float;
   function Midpoint (P1, P2 : Point) return Point;

private
   -- Private declarations visible only to package body
   Default_Origin : constant Point := (0.0, 0.0);
end Geometry;

-- Body (implementation) — geometry.adb
with Ada.Numerics.Elementary_Functions;
use Ada.Numerics.Elementary_Functions;

package body Geometry is

   function Distance (P1, P2 : Point) return Float is
      DX : constant Float := P2.X - P1.X;
      DY : constant Float := P2.Y - P1.Y;
   begin
      return Sqrt (DX * DX + DY * DY);
   end Distance;

   function Midpoint (P1, P2 : Point) return Point is
   begin
      return ((P1.X + P2.X) / 2.0, (P1.Y + P2.Y) / 2.0);
   end Midpoint;

end Geometry;
```

## Generics

```ada
-- Generic package
generic
   type Element_Type is private;
   type Index_Type is range <>;
   type Array_Type is array (Index_Type) of Element_Type;
   with function "<" (Left, Right : Element_Type) return Boolean is <>;
package Sorting is
   procedure Sort (Data : in out Array_Type);
end Sorting;

-- Instantiation
type Int_Array is array (1 .. 100) of Integer;
package Int_Sort is new Sorting (Integer, Integer range 1 .. 100, Int_Array);

-- Generic function
generic
   type T is private;
   with function "=" (A, B : T) return Boolean is <>;
function Generic_Contains (Container : Array_Of_T; Item : T) return Boolean;
```

## Tasking (Concurrency)

```ada
-- Tasks (lightweight threads built into the language)
task type Worker is
   entry Start (Job_Id : Integer);
   entry Get_Result (Result : out Float);
end Worker;

task body Worker is
   My_Id  : Integer;
   Output : Float;
begin
   accept Start (Job_Id : Integer) do
      My_Id := Job_Id;
   end Start;

   -- Do computation
   Output := Compute (My_Id);

   accept Get_Result (Result : out Float) do
      Result := Output;
   end Get_Result;
end Worker;

-- Protected objects (thread-safe shared state)
protected type Bounded_Buffer (Max_Size : Positive) is
   entry Put (Item : Integer);
   entry Get (Item : out Integer);
   function Count return Natural;
private
   Data  : array (1 .. Max_Size) of Integer;
   Size  : Natural := 0;
   Head  : Positive := 1;
   Tail  : Positive := 1;
end Bounded_Buffer;
```

## Exception Handling

```ada
-- Define exceptions
File_Not_Found : exception;
Invalid_Data   : exception;

-- Raise and handle
procedure Load_Config (Path : String) is
   File : File_Type;
begin
   Open (File, In_File, Path);
   -- process file
   Close (File);
exception
   when Name_Error =>
      raise File_Not_Found with "Config not found: " & Path;
   when Data_Error =>
      Close (File);
      raise Invalid_Data with "Corrupt config file";
   when others =>
      if Is_Open (File) then Close (File); end if;
      raise;  -- re-raise
end Load_Config;
```

## Conventions

1. **Use strong typing aggressively.** Create distinct types for distinct concepts. `type Meters is new Float` prevents unit confusion.
2. **Use range constraints** on all numeric types. The compiler generates runtime checks automatically.
3. **Use packages** for encapsulation. Private types hide implementation. Child packages extend hierarchically.
4. **Use tasks and protected objects** for concurrency. Never use OS threads or mutexes directly — Ada's model is formally verifiable.
5. **Use SPARK subset** for safety-critical code. SPARK adds formal verification — prove absence of runtime errors at compile time.
6. **Handle all exceptions** in library packages. Re-raise or translate, but never swallow silently.

---

_Sources: Ada Reference Manual (ada-auth.org), Programming in Ada 2012 (John Barnes), Ada 2022 Language Standard, AdaCore documentation_
