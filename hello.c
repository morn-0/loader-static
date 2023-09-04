#include <stdio.h>
#include <string.h>

int main(int argc, char *argv[], char *envp[]) {
  printf("argc %d\n", argc);

  while (*argv)
    printf("arg %s\n", *argv++);

  while (*envp)
    printf("env %s\n", *envp++);

  return 0;
}
