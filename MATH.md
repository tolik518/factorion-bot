# Mathematical formulas and reasonings
## Exact Factorial
We use the library `rug` (`gmp`).
## Exact Multifactorial
We use the library `rug` (`gmp`).
## Exact Termial
Termials are triangular numbers. They can be calculated with the well-known formula:
```math
n? = \frac{n(n+1)}{2}
```
## Exact Subfactorial
As the most efficient common definition that only uses integers, we use the formula:
```math
\begin{aligned}
!0 &= 1 \\
!n &= n \cdot !(n-1)+(-1)^n
\end{aligned}
```
## Approximate Factorial
Factorials can be approximated with [Stirling’s formula](https://en.wikipedia.org/wiki/Stirling%27s_approximation):
```math
n! \approx \sqrt{2 \pi n} \cdot A(n) \cdot \left(\frac{n}{e}\right)^n
```
Which can be brought into a calculable form with a separate order of magnitude like so:
```math
\begin{aligned}
n! &\approx \sqrt{2 \pi n} \cdot A(n) \cdot \left(\frac{n}{e}\right)^n \text{ | Stirling's Approximation (A(n) only contains negative powers of n, not in exponents)} \\
   &\approx \sqrt{2 \pi n} \cdot A(n) \cdot \left(\frac{n}{e}\right)^m \cdot 10^k \text{ | factoring out the 10 exponent (k)} \\
   &\approx \sqrt{2 \pi n} \cdot A(n) \cdot \left(\frac{n}{e}\right)^{m + \frac{ln(10)}{ln\left(\frac{n}{e}\right)} k} \text{ | factoring it back in (to calculate it)} \\
\\
n &= m + log_{\frac{n}{e}}(10) \cdot k \text{ | got out of exponents} \\
n &\approx log_{\frac{n}{e}}(10) \cdot k \text{ | m should be small} \\
k &= \left\lfloor n / log_{\frac{n}{e}}(10) \right\rfloor \text{ | calculate k which is an integer (floor because otherwhise m < 0)} \\
m &= n - log_{\frac{n}{e}}(10) \cdot k \text{ | calculate the exponent for the calculation}
\end{aligned}
```
## Approximate Multifactorial
We can bring the [continuation](#float-multifactorial) into a calculable form (the major part) like so:
```math
\begin{aligned}
z!_k &= k^{\frac{z}{k}} \cdot \frac{z}{k}! \cdot T_k(z) \text{ | we already have implementations for z! and T_k(z)} \\
\\
k^{\frac{z}{k}} &= k^m \cdot 10^n \
10^{log_{10}(k) \cdot \frac{z}{k}} &= 10^{log_{10}(k) \cdot m} \cdot 10^n \text{ | log_{10}} \\
log_{10}(k) \cdot \frac{z}{k} &= log_{10}(k) \cdot m + n \text{ | n should be as large as possible} \\
\\
n &= \left\lfloor log_{10}(k) \cdot \frac{z}{k} \right\rfloor \\

m \cdot log_{10}(k) &= log_{10}(k) \cdot \frac{z}{k} - n | \div log_{10}(k) \\
m &= \frac{z}{k} - \frac{n}{log_{10}(k)}
\end{aligned}
```
## Approximate Termial
Termials have a simple formula:
```math
n? = \frac{n(n+1)}{2}
```
Which can be brought into a calculable form with a separate order of magnitude like so:
```math
\begin{aligned}
n? &= \frac{n(n+1)}{2} \\
\\
m &= \left\lfloor log_{10}(n) \right\rfloor \\
\\
n? &= k \frac{l}{2} 10^{2m} \\
n? &= k 10^m \cdot l \frac{10^m}{2} \\
\\
k 10^m &= n \\
k &= \frac{n}{10^m} \\
l &= \frac{n+1}{10^m} \\
\end{aligned}
```
## Approximate Subfactorial
A subfactorial is approximately proportional to the factorial:
```math
!n = \left\lfloor \frac{n!+1}{e} \right\rfloor \approx \left\lfloor \frac{n!}{e} \right\rfloor
```
## Approximate Factorial Digits
Factorials can be approximated with [Stirling’s formula](https://en.wikipedia.org/wiki/Stirling%27s_approximation):
```math
n! \approx \sqrt{2 \pi n} \cdot A(n) \cdot (\frac{n}{e})^n
```
Its log_10 can be roughly approximated like so:
```math
\begin{aligned}
log_{10}(n!) &\approx log_{10}(\sqrt{2 \pi n} \cdot \left(\frac{n}{e})^n\right) \text{ | Stirling's Approximation} \\
           &\approx \frac{1}{2} log_{10}(2 \pi n) + n \cdot log_{10}\left(\frac{n}{e}\right) \text{ | splitting up, taking exponents out} \\
           &\approx \frac{1}{2} log_{10}(2 \pi) + \frac{1}{2} log_{10}(n) + n \cdot log_{10}(n) - n \cdot log_{10}(e) \text{ | splitting further} \\
\text{digits} &\approx \left\lfloor \left(\frac{1}{2}+n\right) log_{10}(n) + \frac{1}{2} log_{10}(2 \pi) - \frac{n}{ln(10)} \right\rfloor +1 \text{ | combining log_10(n) and turning into number of digits}
\end{aligned}
```
## Approximate Multifactorial Digits
A k-factorial very roughly is the k-th root of the factorial, while not exact enough for approximations, the number of digits is correct.
## Approximate Termial Digits
Termials have a simple formula:
```math
n? = \frac{n(n+1)}{2}
```
Its log_10 can be roughly approximated like so:
```math
\begin{aligned}
log_{10}(n?) &= log_{10}\left(\frac{n(n+1)}{2}\right) \\
           &= log_{10}\left(n^2+n\right) - log_{10}(2) \text{ | drop inconsequential n} \\
          &\approx 2 log_{10}(n) - log_{10}(2)
\end{aligned}
```
## Approximate Subfactorial Digits
A subfactorial is approximately proportional to the factorial, less than an order of magnitude (just `e`) apart.
The number of digits does not significantly differ.
## Float Factorial
The analytical continuation of factorials is the gamma function, which we use through `rug` (`gmp`):
```math
x! = \Gamma(x+1)
```
## Float Multifactorial
There is an analytical continuation of any k-factorial [here](https://math.stackexchange.com/questions/3488791/define-the-triple-factorial-n-as-a-continuous-function-for-n-in-mathbb/3488935#3488935): 
```math
\begin{aligned}
x!_k &= T_k(x) \cdot k^{\frac{x}{k}} \cdot (\frac{x}{k})! \\
\text{where} \\
T_k(x) &= \prod^k_{j=1}\left(k^{-\frac{j}{k}} j \cdot \left(\frac{j}{k}\right)!^{-1}\right)^{E_{k,j}(x)} \\
\text{where} \\
E_{k,j}(x) &= \frac{1}{k} \sum^k_{l=1}\left(cos(2 \pi l \frac{x-j}{k}\right)
\end{aligned}
```
However this does not match the commonly used double factorial continuation.
To make it match, we have to set:
```math
E_{k,j}(x) = \frac{\prod^{k-1}_{l=0}\left(1 - cos\left(\frac{2}{k} \pi (x-l)\right) \cdot (l \neq j)\right)}{\prod^{k-1}_{l=0}\left(1 - cos\left(-\frac{2}{k} \pi l\right)\right)}
```
Which preserves the trait of equaling 1 for one j while being zero for all others if x is an integer.

To improve performance, we only include those j near x.
## Float Termial
The termial formula is compatible with floats:
```math
n? = \frac{n(n+1)}{2}
```
