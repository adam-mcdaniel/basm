#include <stdio.h>
#include <stdlib.h>
#include <string.h>

int main(int argc, char *argv[]) {
    // Get the output file if any
    FILE *input = stdin;
    FILE *output = stdout;
    char output_base_name[256] = {0};

    if (argc > 1) {
        input = fopen(argv[1], "r");
        if (!input) {
            fprintf(stderr, "Error: Could not open file %s for reading\n", argv[1]);
            return 1;
        } else {
            fprintf(stderr, "Compiling %s\n", argv[1]);
        }

        if (argc > 2) {
            output = fopen(argv[2], "w");
            // Copy everything before the last dot
            char *last_dot = strrchr(argv[2], '.');
            if (last_dot) {
                strncpy(output_base_name, argv[2], last_dot - argv[2]);
            } else {
                strncpy(output_base_name, argv[2], sizeof(output_base_name) - 1);
            }

            if (!output) {
                fprintf(stderr, "Error: Could not open file %s for writing\n", argv[2]);
                return 1;
            } else {
                fprintf(stderr, "Writing to %s\n", argv[2]);
            }
        }
    }

    // Print the header
    fprintf(output, "#include <stdio.h>\n");
    fprintf(output, "#include <stdlib.h>\n");

    // Print the main function
    fprintf(output, "int main(int argc, char *argv[]) {\n");
    // Print the brainfuck code
    fprintf(output, "    unsigned char *tape = calloc(30000, sizeof(char));\n");
    fprintf(output, "    unsigned char *ptr = tape;\n");
    fprintf(output, "    char ch = 0;\n");
    char ch;
    while ((ch = fgetc(input)) != EOF) {
        switch (ch) {
            case '>':
                fprintf(output, "    ptr++;\n");
                break;
            case '<':
                fprintf(output, "    ptr--;\n");
                break;
            case '+':
                fprintf(output, "    (*ptr)++;\n");
                break;
            case '-':
                fprintf(output, "    (*ptr)--;\n");
                break;
            case '.':
                fprintf(output, "    putchar(*ptr);\n");
                break;
            case ',':
                fprintf(output, "    *ptr = (ch = getchar()) == EOF? 0 : ch;\n");
                break;
            case '[':
                fprintf(output, "    while (*ptr) {\n");
                break;
            case ']':
                fprintf(output, "    }\n");
                break;
            case '#':
                // Print the memory of the tape as a hex dump
                fprintf(output, "    for (int i = 0; i < 0x100; i++) {\n");
                fprintf(output, "        if (i %% 16 == 0) {\n");
                fprintf(output, "            printf(\"%%03d-%%03d: \", i, i + 15);\n");
                fprintf(output, "        }\n");
                fprintf(output, "        printf(\"%%02x \", tape[i]);\n");
                fprintf(output, "        if ((i + 1) %% 16 == 0) {\n");
                fprintf(output, "            printf(\"\\n\");\n");
                fprintf(output, "        }\n");
                fprintf(output, "    }\n");
                break;
            case '$':
                // Print the memory of the tape as a decimal dump
                fprintf(output, "    for (int i = 0; i < 0x100; i++) {\n");
                fprintf(output, "        // Print the row number\n");
                fprintf(output, "        if (i %% 16 == 0) {\n");
                fprintf(output, "            printf(\"%%03d-%%03d: \", i, i + 15);\n");
                fprintf(output, "        }\n");
                fprintf(output, "        printf(\"%%3d \", tape[i]);\n");
                fprintf(output, "        if ((i + 1) %% 16 == 0) {\n");
                fprintf(output, "            printf(\"\\n\");\n");
                fprintf(output, "        }\n");
                fprintf(output, "    }\n");
                break;
        }
    }
    fprintf(output, "    free(tape);\n");
    fprintf(output, "    return 0;\n");
    fprintf(output, "}\n");

    fclose(input);
    fclose(output);

    // Compile the output file, if any
    if (argc > 2) {
        char command[1024] = {0};
        snprintf(command, sizeof(command), "gcc -O3 -o %s %s", output_base_name, argv[2]);
        printf("Compiling %s\n", command);
        system(command);
        snprintf(command, sizeof(command), "./%s", output_base_name);
        printf("Running %s\n", command);
        system(command);
    }
}