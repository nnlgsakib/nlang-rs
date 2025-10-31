#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include <math.h>

int main(void);

int main(void) {
    int counter = 0;
    while ((counter < 10)) {
        (counter = (counter + 1));
    if ((counter == 3)) {
        continue;
    }
    if ((counter == 7)) {
        break;
    }
    printf("%d\n", counter);
    }
    return 0;
}

