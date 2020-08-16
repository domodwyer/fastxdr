/*
 * An example XDR spec for test coverage.
 */

const ANSWER = 42;

enum Status {
	GOOD = 1,
	BAD = 2
};

typedef unsigned hyper BIG_NUMBERS;

union choice switch (BIG_NUMBERS num) {
 case ANSWER:
	data       varname;
 default:
	void;
};

struct data {
	BIG_NUMBERS        major;
	int        		   minor;
};