#include <stdlib.h>
#include <stdio.h>
#include <string.h>

typedef unsigned char uchar;
typedef unsigned int uint;

#define NUM_CELLS (32*1024)

void print_now(uchar* o, uint buf_len) {
	fwrite(o, 1, buf_len, stdout);
}

int main() {
	uchar* outb = calloc(/*OUT_BUF_LEN*/, 1);
	uchar* tape = calloc(NUM_CELLS, 1);
	uchar* h = tape;
	
	/*MAIN_CODE*/
	
	free(tape);
	free(outb);
	return 0;
}
