#include <ctype.h>
#include "test.h"

#define T(expr) do { if (!(expr)) t_error(#expr " failed\n"); } while (0)
#define F(expr) do { if ((expr)) t_error(#expr " unexpectedly true\n"); } while (0)

int main(void)
{
	T(isascii(0));
	T(isascii(127));
	F(isascii(128));
	F(isascii(-1));

	T(isdigit('0'));
	T(isdigit('9'));
	F(isdigit('/'));
	F(isdigit(':'));

	T(isxdigit('0'));
	T(isxdigit('f'));
	T(isxdigit('F'));
	F(isxdigit('g'));

	T(isalpha('A'));
	T(isalpha('z'));
	F(isalpha('0'));
	T(isalnum('Q'));
	T(isalnum('7'));
	F(isalnum('_'));

	T(islower('m'));
	F(islower('M'));
	T(isupper('M'));
	F(isupper('m'));

	T(isspace(' '));
	T(isspace('\n'));
	F(isspace('x'));
	T(isblank(' '));
	T(isblank('\t'));
	F(isblank('\n'));

	T(iscntrl(0));
	T(iscntrl(127));
	F(iscntrl(' '));
	T(isprint(' '));
	T(isprint('~'));
	F(isprint(127));
	T(isgraph('!'));
	F(isgraph(' '));
	T(ispunct('!'));
	F(ispunct('A'));

	T(tolower('A') == 'a');
	T(tolower('z') == 'z');
	T(tolower('1') == '1');
	T(toupper('z') == 'Z');
	T(toupper('A') == 'A');
	T(toupper('?') == '?');

	return t_status;
}
