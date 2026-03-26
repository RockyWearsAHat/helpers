# Fortran Conventions and Idioms

## Modern Fortran Philosophy

Modern Fortran (2008/2018/2023) is the dominant language for high-performance scientific computing. It has evolved far beyond FORTRAN 77 — today's Fortran has modules, OOP, coarrays for parallel computing, and rich array operations.

- **Array-first**: Native multi-dimensional arrays with whole-array operations. No library needed.
- **Performance**: Compilers produce highly optimized code. Array semantics enable automatic vectorization.
- **Parallel computing**: Coarrays provide built-in distributed parallelism. OpenMP/MPI for shared/distributed memory.

## Core Syntax

```fortran
program example
    implicit none  ! Always use — prevents undeclared variables

    integer :: i, n
    real(8) :: x, y
    character(len=50) :: name
    logical :: flag

    ! Constants
    real(8), parameter :: PI = 3.141592653589793d0

    ! Arrays
    real(8) :: matrix(3, 3)
    real(8), allocatable :: data(:)

    n = 100
    allocate(data(n))

    ! Array operations (whole-array)
    data = 0.0d0              ! set all elements
    data = [(real(i, 8), i = 1, n)]  ! implied do loop

    ! Element-wise operations
    data = data ** 2 + 1.0d0  ! vectorized
    x = sum(data)
    y = maxval(data)

    deallocate(data)
end program example
```

## Array Operations

```fortran
! Multi-dimensional arrays
real(8) :: A(100, 100), B(100, 100), C(100, 100)

! Whole-array operations
C = A + B                      ! element-wise add
C = matmul(A, B)               ! matrix multiply
x = dot_product(v1, v2)       ! dot product

! Array sections
A(1:10, :) = 0.0d0            ! first 10 rows
A(:, 1) = A(:, 2)             ! copy column

! WHERE (conditional element-wise)
where (A > 0.0d0)
    B = sqrt(A)
elsewhere
    B = 0.0d0
end where

! Array intrinsics
sum(A)                  ! total sum
sum(A, dim=1)          ! column sums (vector)
maxval(A)              ! maximum value
maxloc(A)              ! location of maximum
any(A > 100.0d0)       ! logical: any element > 100?
count(A > 0.0d0)       ! count positive elements
pack(A, A > 0.0d0)     ! extract positive elements
reshape(data, [10, 10]) ! reshape 1D to 2D
```

## Modules

```fortran
module physics
    implicit none
    private                           ! default private
    public :: gravity_force, G        ! explicitly export

    real(8), parameter :: G = 6.674d-11

contains

    pure function gravity_force(m1, m2, r) result(F)
        real(8), intent(in) :: m1, m2, r
        real(8) :: F
        F = G * m1 * m2 / r**2
    end function gravity_force

    pure function kinetic_energy(mass, velocity) result(KE)
        real(8), intent(in) :: mass, velocity
        real(8) :: KE
        KE = 0.5d0 * mass * velocity**2
    end function kinetic_energy

end module physics

! Usage:
program main
    use physics, only: gravity_force, G
    implicit none
    print *, gravity_force(5.97d24, 7.35d22, 3.84d8)
end program main
```

## Procedures

```fortran
! Pure function (no side effects — enables optimization)
pure function factorial(n) result(res)
    integer, intent(in) :: n
    integer :: res, i
    res = 1
    do i = 2, n
        res = res * i
    end do
end function factorial

! Elemental function (auto-vectorized over arrays)
elemental function celsius_to_fahrenheit(c) result(f)
    real(8), intent(in) :: c
    real(8) :: f
    f = c * 9.0d0 / 5.0d0 + 32.0d0
end function celsius_to_fahrenheit

! Subroutine with intent
subroutine solve_linear(A, b, x, info)
    real(8), intent(in)  :: A(:,:)
    real(8), intent(in)  :: b(:)
    real(8), intent(out) :: x(:)
    integer, intent(out) :: info
    ! solve Ax = b
end subroutine solve_linear
```

## OOP

```fortran
module shapes_mod
    implicit none

    type, abstract :: shape
        real(8) :: x = 0.0d0, y = 0.0d0
    contains
        procedure(area_interface), deferred :: area
        procedure :: move
    end type

    abstract interface
        pure function area_interface(self) result(a)
            import :: shape
            class(shape), intent(in) :: self
            real(8) :: a
        end function
    end interface

    type, extends(shape) :: circle
        real(8) :: radius
    contains
        procedure :: area => circle_area
    end type

    type, extends(shape) :: rectangle
        real(8) :: width, height
    contains
        procedure :: area => rectangle_area
    end type

contains

    pure function circle_area(self) result(a)
        class(circle), intent(in) :: self
        real(8) :: a
        a = 3.141592653589793d0 * self%radius**2
    end function

    pure function rectangle_area(self) result(a)
        class(rectangle), intent(in) :: self
        real(8) :: a
        a = self%width * self%height
    end function

    subroutine move(self, dx, dy)
        class(shape), intent(inout) :: self
        real(8), intent(in) :: dx, dy
        self%x = self%x + dx
        self%y = self%y + dy
    end subroutine

end module shapes_mod
```

## Parallel Computing (Coarrays)

```fortran
! Coarrays — built-in distributed parallelism (Fortran 2008)
program parallel_sum
    implicit none
    real(8) :: local_sum, global_sum
    real(8), allocatable :: data(:)[:]  ! coarray
    integer :: me, np, chunk

    me = this_image()      ! process rank (1-based)
    np = num_images()      ! total processes
    chunk = 1000000 / np

    allocate(data(chunk)[*])

    ! Each image works on its chunk
    data = [(real(i + (me-1)*chunk, 8), i = 1, chunk)]
    local_sum = sum(data)

    ! Collective reduction
    call co_sum(local_sum, global_sum)

    if (me == 1) print *, "Total sum:", global_sum
end program parallel_sum
```

## Conventions

1. **Always use `implicit none`.** Prevents silent variable creation from typos.
2. **Declare `intent`** on all procedure arguments: `intent(in)`, `intent(out)`, `intent(inout)`.
3. **Use `pure` and `elemental`** where possible. Enables compiler optimization and auto-vectorization.
4. **Use modules**, not `common` blocks or `include`. Modules provide proper scoping, interfaces, and separate compilation.
5. **Column-major order**: Fortran arrays are column-major. Iterate inner index first: `do j=1,n; do i=1,m; A(i,j)`.
6. **Use `real(8)` or `selected_real_kind`** for floating point. Never use default `real` (often only 32-bit).

---

_Sources: Modern Fortran Explained (Metcalf, Reid, Cohen), fortran-lang.org, Fortran 2018 Standard, Doctor Fortran blog_
