#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include <math.h>

// Helper functions for built-in conversions
char* int_to_str(int value) {
    char* buffer = malloc(32);
    sprintf(buffer, "%d", value);
    return buffer;
}

char* float_to_str(double value) {
    char* buffer = malloc(32);
    sprintf(buffer, "%f", value);
    return buffer;
}

static const char str_const_4[] = "";
static const char str_const_18[] = "sqrt(25)";
static const char str_const_2[] = ": ";
static const char str_const_24[] = "GCD(100, 25)";
static const char str_const_11[] = "Division";
static const char str_const_19[] = "sqrt(10)";
static const char str_const_9[] = "Subtraction";
static const char str_const_5[] = "Basic Arithmetic:";
static const char str_const_10[] = "Multiplication";
static const char str_const_13[] = "2^3";
static const char str_const_17[] = "sqrt(16)";
static const char str_const_14[] = "5^4";
static const char str_const_15[] = "10^0";
static const char str_const_1[] = "Error: Cannot calculate square root of negative number!";
static const char str_const_28[] = "Approximate area of circle (r=5): ";
static const char str_const_26[] = "Complex Calculations:";
static const char str_const_29[] = "Calculator operations completed!";
static const char str_const_27[] = "(12 * 8) + (5 / 2) = ";
static const char str_const_8[] = "Addition";
static const char str_const_20[] = "Prime Number Checking:";
static const char str_const_0[] = "Error: Division by zero!";
static const char str_const_21[] = " is prime";
static const char str_const_22[] = "Greatest Common Divisor:";
static const char str_const_12[] = "Power Calculations:";
static const char str_const_6[] = "First number";
static const char str_const_25[] = "GCD(17, 13)";
static const char str_const_23[] = "GCD(48, 18)";
static const char str_const_3[] = "=== Advanced Calculator ===";
static const char str_const_7[] = "Second number";
static const char str_const_16[] = "Square Root Approximations:";

int add(int a, int b);
int subtract(int a, int b);
int multiply(int a, int b);
double divide(int a, int b);
int power(int base, int exponent);
double sqrt_approx(int number);
int is_prime(int n);
int gcd(int a, int b);
void display_result(const char* label, int value);
void display_result_float(const char* label, double value);
int main(void);

int add(int a, int b) {
    return (a + b);
}

int subtract(int a, int b) {
    return (a - b);
}

int multiply(int a, int b) {
    return (a * b);
}

double divide(int a, int b) {
    if ((b == 0)) {
        printf("%s\n", str_const_0);
    return 0;
    }
    return (a / b);
}

int power(int base, int exponent) {
    if ((exponent == 0)) {
        return 1;
    }
    int result = 1;
    int i = 0;
    while ((i < exponent)) {
        (result = (result * base));
    (i = (i + 1));
    }
    return result;
}

double sqrt_approx(int number) {
    if ((number < 0)) {
        printf("%s\n", str_const_1);
    return 0;
    }
    if ((number == 0)) {
        return 0;
    }
    int guess = (atof(int_to_str(number)) / 2);
    double precision = 0.001;
    int iterations = 0;
    while ((iterations < 20)) {
        int new_guess = ((guess + (atof(int_to_str(number)) / guess)) / 2);
    int difference = fabs((new_guess - guess));
    if ((difference < precision)) {
        return new_guess;
    }
    (guess = new_guess);
    (iterations = (iterations + 1));
    }
    return guess;
}

int is_prime(int n) {
    if ((n <= 1)) {
        return 0;
    }
    if ((n <= 3)) {
        return 1;
    }
    if ((((n % 2) == 0) || ((n % 3) == 0))) {
        return 0;
    }
    int i = 5;
    while (((i * i) <= n)) {
        if ((((n % i) == 0) || ((n % (i + 2)) == 0))) {
        return 0;
    }
    (i = (i + 6));
    }
    return 1;
}

int gcd(int a, int b) {
    while ((b != 0)) {
        int temp = b;
    (b = (a % b));
    (a = temp);
    }
    return a;
}

void display_result(const char* label, int value) {
    printf("%s", label);
    printf("%s", str_const_2);
    printf("%s\n", int_to_str(value));
}

void display_result_float(const char* label, double value) {
    printf("%s", label);
    printf("%s", str_const_2);
    printf("%s\n", int_to_str(value));
}

int main(void) {
    printf("%s\n", str_const_3);
    printf("%s\n", str_const_4);
    printf("%s\n", str_const_5);
    int num1 = 15;
    int num2 = 4;
    display_result(str_const_6, num1);
    display_result(str_const_7, num2);
    printf("%s\n", str_const_4);
    display_result(str_const_8, add(num1, num2));
    display_result(str_const_9, subtract(num1, num2));
    display_result(str_const_10, multiply(num1, num2));
    display_result_float(str_const_11, divide(num1, num2));
    printf("%s\n", str_const_4);
    printf("%s\n", str_const_12);
    display_result(str_const_13, power(2, 3));
    display_result(str_const_14, power(5, 4));
    display_result(str_const_15, power(10, 0));
    printf("%s\n", str_const_4);
    printf("%s\n", str_const_16);
    display_result_float(str_const_17, sqrt_approx(16));
    display_result_float(str_const_18, sqrt_approx(25));
    display_result_float(str_const_19, sqrt_approx(10));
    printf("%s\n", str_const_4);
    printf("%s\n", str_const_20);
    int numbers_to_check = 17;
    int i = 2;
    while ((i <= numbers_to_check)) {
        int prime_result = is_prime(i);
    if (prime_result) {
        printf("%s", int_to_str(i));
    printf("%s\n", str_const_21);
    }
    (i = (i + 1));
    }
    printf("%s\n", str_const_4);
    printf("%s\n", str_const_22);
    display_result(str_const_23, gcd(48, 18));
    display_result(str_const_24, gcd(100, 25));
    display_result(str_const_25, gcd(17, 13));
    printf("%s\n", str_const_4);
    printf("%s\n", str_const_26);
    int a = 12;
    int b = 8;
    int c = 5;
    int complex_result = add(multiply(a, b), ((int)divide(c, 2)));
    printf("%s", str_const_27);
    printf("%s\n", int_to_str(complex_result));
    int area_circle = multiply(multiply(3, 14), multiply(5, 5));
    printf("%s", str_const_28);
    printf("%s\n", int_to_str(area_circle));
    printf("%s\n", str_const_4);
    printf("%s\n", str_const_29);
    return 0;
}

