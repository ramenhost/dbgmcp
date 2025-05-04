#include <stdio.h>
#include <stdlib.h>

void function_that_crashes() {
    // Cause segmentation fault
    char *ptr = "static string";
    ptr[0] = 42;  // This will cause a crash
}

void function_with_args(int a, int b) {
    printf("Arguments: a=%d, b=%d\n", a, b);
    if (a > 10) {
        function_that_crashes();
    }
}

int main(int argc, char **argv) {
    printf("Starting the program...\n");

    // Print command line arguments
    printf("Got %d arguments\n", argc);
    for (int i = 0; i < argc; i++) {
        printf("Argument %d: %s\n", i, argv[i]);
    }

    int number = 5;

    if (argc > 1) {
        number = atoi(argv[1]);
    }

    printf("Working with number: %d\n", number);

    function_with_args(number, number * 2);

    printf("Program completed successfully\n");
    return 0;
}
