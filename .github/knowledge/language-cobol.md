# COBOL Best Practices

## COBOL Context

COBOL powers the world's financial infrastructure. Banks, insurance companies, and government agencies run trillions of dollars in daily transactions on COBOL. Understanding it is essential for modernization, maintenance, and integration of legacy systems.

- **Business-oriented**: Self-documenting syntax that reads like English. Designed for business data processing.
- **Fixed-point decimal**: Native packed decimal arithmetic — no floating-point rounding errors for money.
- **Batch processing**: Built for high-volume sequential file processing and report generation.

## Program Structure

```cobol
       IDENTIFICATION DIVISION.
       PROGRAM-ID. CUSTOMER-REPORT.
       AUTHOR. MAINTENANCE-TEAM.
      *> Modern COBOL supports *> for comments

       ENVIRONMENT DIVISION.
       INPUT-OUTPUT SECTION.
       FILE-CONTROL.
           SELECT CUSTOMER-FILE
               ASSIGN TO 'CUSTFILE'
               ORGANIZATION IS INDEXED
               ACCESS MODE IS DYNAMIC
               RECORD KEY IS CUST-ID
               FILE STATUS IS WS-FILE-STATUS.

           SELECT REPORT-FILE
               ASSIGN TO 'CUSTRPT'
               ORGANIZATION IS SEQUENTIAL.

       DATA DIVISION.
       FILE SECTION.
       FD CUSTOMER-FILE.
       01 CUSTOMER-RECORD.
           05 CUST-ID              PIC 9(8).
           05 CUST-NAME            PIC X(30).
           05 CUST-BALANCE         PIC S9(7)V99 COMP-3.
           05 CUST-STATUS          PIC X(1).
               88 CUST-ACTIVE      VALUE 'A'.
               88 CUST-INACTIVE    VALUE 'I'.

       WORKING-STORAGE SECTION.
       01 WS-FILE-STATUS           PIC XX.
       01 WS-EOF-FLAG              PIC X VALUE 'N'.
           88 END-OF-FILE          VALUE 'Y'.
       01 WS-TOTAL-BALANCE         PIC S9(10)V99 VALUE ZEROS.
       01 WS-RECORD-COUNT          PIC 9(6) VALUE ZEROS.

       PROCEDURE DIVISION.
       MAIN-PROCESS.
           PERFORM INITIALIZE-FILES
           PERFORM PROCESS-RECORDS UNTIL END-OF-FILE
           PERFORM FINALIZE-REPORT
           STOP RUN.
```

## Data Types (PICTURE Clauses)

```cobol
      *> Numeric types
       01 WS-INTEGER         PIC 9(5).           *> 00000-99999
       01 WS-SIGNED          PIC S9(5).          *> -99999 to +99999
       01 WS-DECIMAL         PIC 9(5)V99.        *> 00000.00 to 99999.99
       01 WS-PACKED          PIC S9(7)V99 COMP-3. *> packed decimal (BCD)
       01 WS-BINARY          PIC S9(9) COMP.      *> binary integer

      *> String types
       01 WS-NAME            PIC X(30).           *> 30 alphanumeric chars
       01 WS-CODE            PIC A(5).            *> 5 alphabetic chars

      *> Edited (display formatting)
       01 WS-FORMATTED-AMT   PIC $$$,$$9.99-.     *> $  1,234.56
       01 WS-DATE-DISPLAY    PIC 9999/99/99.      *> 2024/03/15
       01 WS-PERCENT         PIC ZZ9.99%.         *>  95.50%

      *> Group items (like structs)
       01 WS-ADDRESS.
           05 WS-STREET      PIC X(40).
           05 WS-CITY        PIC X(20).
           05 WS-STATE       PIC X(2).
           05 WS-ZIP         PIC 9(5).

      *> 88-level condition names (enums)
       01 WS-TRANSACTION-TYPE PIC X(1).
           88 IS-DEPOSIT      VALUE 'D'.
           88 IS-WITHDRAWAL   VALUE 'W'.
           88 IS-TRANSFER     VALUE 'T'.
           88 IS-VALID-TYPE   VALUE 'D' 'W' 'T'.
```

## Procedure Patterns

```cobol
       PROCESS-RECORDS.
           READ CUSTOMER-FILE
               AT END SET END-OF-FILE TO TRUE
               NOT AT END PERFORM PROCESS-ONE-CUSTOMER
           END-READ.

       PROCESS-ONE-CUSTOMER.
           IF CUST-ACTIVE
               ADD CUST-BALANCE TO WS-TOTAL-BALANCE
               ADD 1 TO WS-RECORD-COUNT
               PERFORM WRITE-REPORT-LINE
           END-IF.

      *> EVALUATE (switch/case)
       DETERMINE-ACTION.
           EVALUATE TRUE
               WHEN IS-DEPOSIT
                   PERFORM PROCESS-DEPOSIT
               WHEN IS-WITHDRAWAL
                   PERFORM PROCESS-WITHDRAWAL
               WHEN IS-TRANSFER
                   PERFORM PROCESS-TRANSFER
               WHEN OTHER
                   PERFORM HANDLE-INVALID-TYPE
           END-EVALUATE.

      *> PERFORM variations
           PERFORM PROCESS-ITEM
               VARYING WS-IDX FROM 1 BY 1
               UNTIL WS-IDX > WS-TABLE-SIZE.

      *> Inline PERFORM
           PERFORM UNTIL END-OF-FILE
               READ INPUT-FILE
                   AT END SET END-OF-FILE TO TRUE
               END-READ
               IF NOT END-OF-FILE
                   PERFORM PROCESS-RECORD
               END-IF
           END-PERFORM.
```

## File Processing

```cobol
       INITIALIZE-FILES.
           OPEN INPUT CUSTOMER-FILE
           OPEN OUTPUT REPORT-FILE
           IF WS-FILE-STATUS NOT = '00'
               DISPLAY 'FILE OPEN ERROR: ' WS-FILE-STATUS
               STOP RUN
           END-IF.

      *> Sequential read
       READ-NEXT.
           READ CUSTOMER-FILE NEXT RECORD
               AT END SET END-OF-FILE TO TRUE
           END-READ.

      *> Indexed read (random access)
       READ-BY-KEY.
           MOVE 12345678 TO CUST-ID
           READ CUSTOMER-FILE
               INVALID KEY
                   DISPLAY 'CUSTOMER NOT FOUND'
               NOT INVALID KEY
                   PERFORM DISPLAY-CUSTOMER
           END-READ.

      *> Write
       WRITE-OUTPUT.
           WRITE REPORT-RECORD FROM WS-OUTPUT-LINE
               AFTER ADVANCING 1 LINE.

       FINALIZE-REPORT.
           CLOSE CUSTOMER-FILE
           CLOSE REPORT-FILE.
```

## String Handling

```cobol
      *> STRING (concatenation)
           STRING WS-FIRST-NAME DELIMITED BY SPACES
                  ' '            DELIMITED BY SIZE
                  WS-LAST-NAME   DELIMITED BY SPACES
                  INTO WS-FULL-NAME
           END-STRING.

      *> UNSTRING (split)
           UNSTRING WS-CSV-LINE
               DELIMITED BY ','
               INTO WS-FIELD-1 WS-FIELD-2 WS-FIELD-3
               TALLYING IN WS-FIELD-COUNT
           END-UNSTRING.

      *> INSPECT (search/replace)
           INSPECT WS-TEXT
               TALLYING WS-COUNT FOR ALL 'ERROR'
               REPLACING ALL 'ERROR' BY 'WARN '.

      *> Reference modification (substring)
           MOVE WS-DATE(1:4) TO WS-YEAR.
           MOVE WS-DATE(5:2) TO WS-MONTH.
```

## Key Rules

1. **Always check FILE STATUS** after every file operation. Don't assume I/O succeeds.
2. **Use `COMP-3` (packed decimal)** for financial calculations. Never use COMP (binary) for money.
3. **Use 88-level conditions** for status flags and enums. `IF CUST-ACTIVE` is clearer than `IF CUST-STATUS = 'A'`.
4. **Use structured programming.** `PERFORM ... END-PERFORM`, `IF ... END-IF`, `EVALUATE ... END-EVALUATE`. Never use `GO TO`.
5. **Initialize all working storage.** Use `INITIALIZE` verb or `VALUE` clauses. Uninitialized data contains garbage.
6. **Use meaningful paragraph names.** COBOL is self-documenting by design — `PROCESS-CUSTOMER-PAYMENT` not `PARA-1`.

---

_Sources: Enterprise COBOL for z/OS Programming Guide (IBM), Beginning COBOL for Programmers (Coughlan), COBOL Standards (ANSI/ISO), Micro Focus COBOL documentation_
