#include <stdlib.h>
#include <stdio.h>
#include <string.h>

typedef unsigned char uchar;
typedef unsigned int uint;

#define NUM_CELLS (64*1024)

void print_now(uchar* o, uint buf_len) {
	fwrite(o, 1, buf_len, stdout);
}

int main() {
	uchar outb[/*OUT_BUF_LEN*/] = {0};
	uchar* tape = calloc(NUM_CELLS, 1);
	uchar* h = tape;
	
	/*MAIN_CODE*/
	
	// don't free tape, let it be reclaimed by process exit
	return 0;
}
