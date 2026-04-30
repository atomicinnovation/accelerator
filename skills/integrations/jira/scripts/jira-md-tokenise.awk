#!/usr/bin/env awk -f
# jira-md-tokenise.awk — Pass 1: Markdown block tokeniser (POSIX awk)
#
# Reads Markdown on stdin. Emits a binary record stream where:
#   FS between fields = ASCII Unit Separator (\x1f = )
#   RS terminating each record = ASCII Record Separator (\x1e = )
#
# Record types (fields separated by \x1f, records terminated by \x1e):
#   P      <US> <text>           paragraph segment
#   H<n>   <US> <text>           heading level n (1–6)
#   BUL    <US> <text>           bullet list item
#   ORD    <US> <n> <US> <text>  ordered list item
#   TASK_TODO <US> <text>        unchecked checklist item
#   TASK_DONE <US> <text>        checked checklist item
#   CODE_OPEN <US> <lang>        fenced code block open
#   CODE_LINE <US> <text>        literal code line
#   CODE_CLOSE                   fenced code block close
#   HBR                          hard break — next P is continuation of current paragraph
#   ERR    <US> <E_CODE> <US> <msg>  rejection (tokeniser also writes to stderr)
#
# Hard-break semantics: a line with 2+ trailing spaces emits the accumulated
# paragraph segment as P, then emits HBR. The next non-blank line continues
# that paragraph. The assembler merges P, HBR, P sequences into one paragraph.
#
# Pre-validation rejections (exit 41):
#   E_ADF_UNSUPPORTED_TABLE, E_ADF_UNSUPPORTED_NESTED_LIST,
#   E_ADF_UNSUPPORTED_BLOCKQUOTE
# Inputs containing \x1e or \x1f bytes exit 42 (E_ADF_BAD_INPUT).

BEGIN {
    US = "\x1f"
    RS_OUT = "\x1e"
    in_code = 0
    in_para = 0
    para_text = ""
    last_hbr = 0
    error_seen = 0
}

{
    line = $0
    sub(/\r$/, "", line)

    if (line ~ /\x1e/ || line ~ /\x1f/) {
        emit_err("E_ADF_BAD_INPUT", "input contains control byte \\x1e or \\x1f")
        error_seen = 42
        exit 42
    }

    # Inside fenced code block
    if (in_code) {
        if (line ~ /^```/) {
            emit("CODE_CLOSE", "")
            in_code = 0
        } else {
            emit("CODE_LINE" US line, "")
        }
        next
    }

    # Blank line — flush paragraph
    if (line ~ /^[[:space:]]*$/) {
        flush_para()
        last_hbr = 0
        next
    }

    # Pre-validation: blockquote
    if (line ~ /^>/) {
        flush_para()
        emit_err("E_ADF_UNSUPPORTED_BLOCKQUOTE", "blockquote is not supported")
        error_seen = 41; exit 41
    }

    # Pre-validation: pipe table
    if (line ~ /^\|/ && line ~ /\|[[:space:]]*$/) {
        flush_para()
        emit_err("E_ADF_UNSUPPORTED_TABLE", "pipe tables are not supported")
        error_seen = 41; exit 41
    }

    # Pre-validation: nested list (indented list marker)
    if (line ~ /^[[:space:]]+[-*+]/ || line ~ /^[[:space:]]+[0-9]+\./) {
        flush_para()
        emit_err("E_ADF_UNSUPPORTED_NESTED_LIST", "nested lists are not supported")
        error_seen = 41; exit 41
    }

    # Fenced code block open
    if (line ~ /^```/) {
        flush_para()
        lang = line; sub(/^```/, "", lang)
        emit("CODE_OPEN" US lang, "")
        in_code = 1; next
    }

    # ATX heading
    if (line ~ /^#{1,6} /) {
        flush_para()
        level = 0; tmp = line
        while (substr(tmp, 1, 1) == "#") { level++; tmp = substr(tmp, 2) }
        sub(/^[[:space:]]+/, "", tmp)
        emit("H" level US tmp, "")
        next
    }

    # List items (bullet, task, ordered)
    if (line ~ /^[-*+] /) {
        flush_para()
        text = line; sub(/^[-*+][[:space:]]+/, "", text)
        if (text ~ /^\[ \] /) {
            sub(/^\[ \] /, "", text); emit("TASK_TODO" US text, "")
        } else if (text ~ /^\[[xX]\] /) {
            sub(/^\[[xX]\] /, "", text); emit("TASK_DONE" US text, "")
        } else {
            emit("BUL" US text, "")
        }
        next
    }

    if (line ~ /^[0-9]+\. /) {
        flush_para()
        n = line; sub(/\. .*$/, "", n)
        text = line; sub(/^[0-9]+\. /, "", text)
        emit("ORD" US n US text, "")
        next
    }

    # Paragraph accumulation (default rule)
    {
        if (line ~ /__/)
            print "Notice: '__...__' is not emphasis in this subset; use **...** for bold" > "/dev/stderr"

        # If hard-break pending: emit current segment as P, then HBR
        if (in_para && last_hbr) {
            emit_para_seg()
            emit("HBR", "")
        }
        last_hbr = 0

        # Detect trailing spaces (hard break marker) on this line
        if (line ~ /  +$/) {
            sub(/[[:space:]]+$/, "", line)
            last_hbr = 1
        }

        if (in_para && para_text != "") {
            para_text = para_text " " line
        } else {
            para_text = line
            in_para = 1
        }
    }
}

END {
    if (!error_seen) flush_para()
}

function emit_para_seg(    t) {
    if (para_text != "") {
        t = para_text; para_text = ""
        emit("P" US t, "")
    }
}

function flush_para() {
    emit_para_seg()
    in_para = 0; last_hbr = 0
}

function emit(rec, _dummy) { printf "%s%s", rec, RS_OUT }

function emit_err(code, msg) {
    printf "ERR%s%s%s%s%s", US, code, US, msg, RS_OUT
    print code ": " msg > "/dev/stderr"
}
