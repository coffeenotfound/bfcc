#include <stdlib.h>
#include <stdio.h>
#include <string.h>

typedef unsigned char uchar;
typedef unsigned int uint;

#define NUM_CELLS (32*1024)

uchar *o;
uchar *b;
uchar *c;

void print_now(uint buf_len) {
	fwrite(o, 1, buf_len, stdout);
}

int main() {
	o = calloc(/*OUT_BUF_LEN*/, 1);
	b = calloc(NUM_CELLS, 1);
	c = b;
	
	/*MAIN_CODE*/
	
	free(b);
	return 0;
}
