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
	uchar* o = calloc(/*OUT_BUF_LEN*/, 1);
	uchar* b = calloc(NUM_CELLS, 1);
	uchar* c = b;
	
	/*MAIN_CODE*/
	
	free(b);
	return 0;
}
