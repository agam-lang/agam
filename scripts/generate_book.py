#!/usr/bin/env python3
import os
import re
import sys
from pathlib import Path

DOC_DIR = Path("c:/Users/ksvik/Projects/Agam-Lang/doc")
SUMMARY_FILE = DOC_DIR / "SUMMARY.md"
OUTPUT_MD = DOC_DIR / "Agam_Compiler_Book.md"
OUTPUT_PDF = DOC_DIR / "Agam_Compiler_Book.pdf"

def clean_markdown_content(content: str) -> str:
    """Removes IDE-specific file:/// links and normalizes headers/code blocks."""
    # Replace file:///c:/Users/ksvik/Projects/Agam-Lang/doc/... links with internal anchor or text
    content = re.sub(r'\[([^\]]+)\]\(file:///[^\)]+\)', r'\1', content)
    # Remove metadata comments or HTML tags if needed
    return content

def compile_unified_markdown():
    """Combines all chapters in SUMMARY.md into a single manuscript."""
    print("Reading SUMMARY.md...")
    with open(SUMMARY_FILE, "r", encoding="utf-8") as f:
        summary_lines = f.readlines()

    chapter_paths = []
    for line in summary_lines:
        match = re.search(r'\(([^)]+\.md)\)', line)
        if match:
            rel_path = match.group(1)
            full_path = DOC_DIR / rel_path
            if full_path.exists() and full_path != OUTPUT_MD:
                chapter_paths.append((rel_path, full_path))

    print(f"Found {len(chapter_paths)} chapter files.")

    combined_md = []
    combined_md.append("# Engineering the Agam Compiler & Language Programming Guide\n\n")
    combined_md.append("*A Complete Textbook, Architecture Reference & Language User Guide*\n\n")
    combined_md.append("---\n\n")

    for rel_path, full_path in chapter_paths:
        if full_path.name == "SUMMARY.md":
            continue
        print(f"Processing {rel_path}...")
        with open(full_path, "r", encoding="utf-8") as f:
            text = f.read()
        cleaned = clean_markdown_content(text)
        combined_md.append(cleaned)
        combined_md.append("\n\n---\n\npagebreak\n\n")

    with open(OUTPUT_MD, "w", encoding="utf-8") as f:
        f.write("".join(combined_md))

    print(f"Successfully generated unified manuscript at: {OUTPUT_MD}")

def generate_pdf_from_md():
    """Generates a styled PDF from the unified markdown using ReportLab."""
    try:
        from reportlab.lib.pagesizes import letter
        from reportlab.platypus import SimpleDocTemplate, Paragraph, Spacer, PageBreak, HRFlowable, Preformatted
        from reportlab.lib.styles import getSampleStyleSheet, ParagraphStyle
        from reportlab.lib import colors
    except ImportError:
        print("ReportLab is not installed yet. Skipping PDF generation.")
        return

    print("Building PDF using ReportLab...")
    
    with open(OUTPUT_MD, "r", encoding="utf-8") as f:
        text = f.read()

    doc = SimpleDocTemplate(
        str(OUTPUT_PDF),
        pagesize=letter,
        rightMargin=54,
        leftMargin=54,
        topMargin=54,
        bottomMargin=54
    )

    styles = getSampleStyleSheet()
    
    # Custom styles
    title_style = ParagraphStyle(
        'BookTitle',
        parent=styles['Heading1'],
        fontSize=24,
        leading=28,
        textColor=colors.HexColor("#1e293b"),
        spaceAfter=15,
    )
    
    h1_style = ParagraphStyle(
        'BookH1',
        parent=styles['Heading1'],
        fontSize=18,
        leading=22,
        textColor=colors.HexColor("#0f172a"),
        spaceBefore=15,
        spaceAfter=10,
    )

    h2_style = ParagraphStyle(
        'BookH2',
        parent=styles['Heading2'],
        fontSize=14,
        leading=18,
        textColor=colors.HexColor("#334155"),
        spaceBefore=12,
        spaceAfter=8,
    )

    body_style = ParagraphStyle(
        'BookBody',
        parent=styles['Normal'],
        fontSize=10,
        leading=14,
        textColor=colors.HexColor("#1e293b"),
        spaceAfter=8,
    )

    code_style = ParagraphStyle(
        'BookCode',
        parent=styles['Code'],
        fontSize=8.5,
        leading=11,
        fontName='Courier',
        textColor=colors.HexColor("#0f172a"),
        backColor=colors.HexColor("#f1f5f9"),
        borderColor=colors.HexColor("#cbd5e1"),
        borderWidth=0.5,
        borderPadding=6,
        spaceBefore=8,
        spaceAfter=8,
    )

    story = []
    
    lines = text.split("\n")
    in_code_block = False
    code_lines = []

    for line in lines:
        if line.startswith("```"):
            if in_code_block:
                # End code block
                code_text = "\n".join(code_lines)
                story.append(Preformatted(code_text, code_style))
                code_lines = []
                in_code_block = False
            else:
                # Start code block
                in_code_block = True
                code_lines = []
            continue

        if in_code_block:
            code_lines.append(line)
            continue

        if line.startswith("pagebreak"):
            story.append(PageBreak())
            continue

        if line.startswith("---"):
            story.append(HRFlowable(width="100%", thickness=0.5, color=colors.HexColor("#cbd5e1"), spaceBefore=10, spaceAfter=10))
            continue

        if not line.strip():
            story.append(Spacer(1, 4))
            continue

        # Escape HTML entities for reportlab
        safe_line = line.replace('&', '&amp;').replace('<', '&lt;').replace('>', '&gt;')
        # Bold formatting **text**
        safe_line = re.sub(r'\*\*(.*?)\*\*', r'<b>\1</b>', safe_line)
        # Italic formatting *text*
        safe_line = re.sub(r'\*(.*?)\*', r'<i>\1</i>', safe_line)
        # Inline code `text`
        safe_line = re.sub(r'`(.*?)`', r'<font name="Courier" color="#0f172a">\1</font>', safe_line)

        if line.startswith("# "):
            story.append(Paragraph(safe_line[2:], title_style))
        elif line.startswith("## "):
            story.append(Paragraph(safe_line[3:], h1_style))
        elif line.startswith("### "):
            story.append(Paragraph(safe_line[4:], h2_style))
        else:
            story.append(Paragraph(safe_line, body_style))

    doc.build(story)
    print(f"Successfully compiled PDF at: {OUTPUT_PDF}")

if __name__ == "__main__":
    compile_unified_markdown()
    generate_pdf_from_md()
