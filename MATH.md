# Mathematical formulas and resonings
## Exact Factorial
We use the library `rug` (`gmp`).
## Exact Multifactorial
We use the library `rug` (`gmp`).
## Exact Termial
Termials are triangular numbers, they can be calculated with the well-known formula:
```
n? = (n*(n+1))/2
```
## Exact Subfactorial
As the most efficient common definition, that only uses integers, we use the formula:
```
!0 = 1
!n = n*!(n-1)+(-1)^n 
```
## Approximate Factorial
Factorials can be approximated with sterlings formula:
```
n! ~= sqrt(2pi*n) * A(n) * (n/e)^n
```
Which can be brought into a calculable form with separate order of magnitude like so:
```
n! ~= sqrt(2pi*n) * A(n) * (n/e)^n |Stirling's Approximation (A(n) only contains negative powers of n, not in exponents)
   ~= sqrt(2pi*n) * A(n) * (n/e)^m * 10^k |factoring out the 10 exponent (k)
   ~= sqrt(2pi*n) * A(n) * (n/e)^(m + ln(10)/ln(n/e)*k) |factoring it back in (to calculate it)

n  = m + log_(n/e)(10)*k |got out of exponents
n ~= log_(n/e)(10)*k |m should be small
k  = floor(n / log_(n/e)(10)) |calculate k which is an integer (floor becaus otherwhise m < 0)
m  = n - log_(n/e)(10)*k |calculate the exponent for the calculation
```
## Approximate Multifactorial

```
z!_k = k^(z/k) * (z/k)! * T_k(z) | we already have implementations for z! and T_k(z)

k^(z/k)             = k^m * 10^n
10^(log10(k) * z/k) = 10^(log10(k)*m) * 10^n | log10
log10(k) * z/k      = log10(k)*m + n | n should be as large as possible

n = floor(log10(k) * z/k)

m*log10(k) = log10(k) * z/k - n | /log10(k)
m          = z/k - n/log10(k)
```
## Approximate Termial
Termials have a simple formula:
```
n? = (n*(n+1))/2
```
Which can be brought into a calculable form with separate order of magnitude like so:
```
n? = n*(n+1)/2

m = floor(log10(n))

n? = k*l/2 * 10^2m
n? = k*10^m * l*10^m/2

k*10^m = n
k = n/10^m
l = (n+1)/10^m
```
## Approximate Subfactorial
A subfactorial is approximatly proportional to the factorial:
```
!n ~= floor(n!/e)
```
## Approximate Factorial Digits
Factorials can be approximated with sterlings formula:
```
n! ~= sqrt(2pi*n) * A(n) * (n/e)^n
```
Its log_10 can be roughly approximated like so:
```
log_10(n!) ~= log_10(sqrt(2pi*n) * (n/e)^n) |Sterling's Approximation
           ~= 1/2*log_10(2pi*n) + n*log_10(n/e) |splitting up, taking exponents out
           ~= 1/2*log_10(2pi) + 1/2*log_10(n) + n*log_10(n) - n*log_10(e) |splitting further
digits     ~= floor((1/2+n)*log_10(n) + 1/2*log_10(2pi) - n/ln(10))+1 |combining log_10(n) and turning into number of digits
```
## Approximate Multifactorial Digits
A k-factorial very roughly is the k-th root of the factorial, while not exact enough for approximations, the number of digits is correct.
## Approximate Termial Digits
Termials have a simple formula:
```
n? = (n*(n+1))/2
```
Its log_10 can be roughly approximated like so:
```
log10(n?)  = log10((n*(n+1))/2)
           = log10(n^2+n) - log10(2) | drop inconsequential n
          ~= 2*log10(n) - log10(2)
```
## Approximate Subfactorial Digits
A subfactorial is approximatly proportional to the factorial, less than an order of magnitude (just `e`) apart.
The number of digits does not significantly differ.
## Float Factorial
The analytical continuation of factorials is the gamma function, which we use through `rug` (`gmp`):
```
x! = gamma(x+1)
```
## Float Multifactorial
There is an analytical continuation of any k-factorial [here](https://math.stackexchange.com/questions/3488791/define-the-triple-factorial-n-as-a-continuous-function-for-n-in-mathbb/3488935#3488935): 
```
x!_(k) = T_k(x) * k^(x/k) * (x/k)!
where
T_k(x) = prod^k_(j=1)(j * k^(-j/k) / (j/k)!)^E_(k,j)(x)
where
E_(k,j)(x) = 1/k * sum^k_(l=1)(cos(2*pi*l*(x-j)/k))
```
However this does not match the commonly (WolframAlpha) used Doublefactorial continuation.
To make it match we have to set:
```
E_(k,j)(x) = prod^(k-1)_(l=0)(1 - cos(2/k*pi*(x-l)) * (l!=j))/prod^(k-1)_(l=0)(1 - cos(-2/k*pi*l))
```
Which preserves the trait, of equaling 1 for one j, while being zero for all others if x is an integer.

To improve performance, we only include those j near x.
## Float Termial
The termial formula is compatible with floats:
```
n? = (n*(n+1))/2
```
