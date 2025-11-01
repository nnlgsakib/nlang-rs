#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include <math.h>

static const char str_const_9[] = "Is graduated: ";
static const char str_const_6[] = "Pi: ";
static const char str_const_5[] = "Year: ";
static const char str_const_0[] = "Alice";
static const char str_const_3[] = "Name: ";
static const char str_const_7[] = "Temperature: ";
static const char str_const_1[] = "Hello, nlang!";
static const char str_const_4[] = "Age: ";
static const char str_const_2[] = "=== Variable Examples ===";
static const char str_const_8[] = "Is student: ";

int main(void);

int main(void) {
    int age = 25;
    int year = 2024;
    int pi = 3.14159;
    int temperature = 98.6;
    int is_student = 1;
    int is_graduated = 0;
    int name = str_const_0;
    int greeting = str_const_1;
    printf("%s\n", str_const_2);
    printf("%s", str_const_3);
    printf("%d\n", name);
    printf("%s", str_const_4);
    printf("%d\n", age);
    printf("%s", str_const_5);
    printf("%d\n", year);
    printf("%s", str_const_6);
    printf("%d\n", pi);
    printf("%s", str_const_7);
    printf("%d\n", temperature);
    printf("%s", str_const_8);
    printf("%d\n", is_student);
    printf("%s", str_const_9);
    printf("%d\n", is_graduated);
    printf("%d\n", greeting);
    return 0;
}

