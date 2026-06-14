using System;
using System.Collections.Generic;
using System.Text;
using Nordocs.Ffi;

namespace Nordocs.Ffi.ConsumerTest
{
    /// <summary>
    /// Minimal round-trip over the real <c>nordocs</c> cdylib through the C#
    /// bindings: a Markdown conversion, a PDF compile (with and without
    /// <c>sys.inputs</c>), a multi-format SVG compile, a source-map session, and the
    /// error path. Exits non-zero on any failure so <c>run.sh</c> / CI can gate on it.
    /// </summary>
    internal static class Program
    {
        private static int Main()
        {
            try
            {
                // IMarkdownToTypstConverter.Convert
                string typ = Typst.MarkdownToTypst("# Title\n\nBody text");
                Require(typ.Contains("Title"), "markdown conversion preserves the heading text");

                // ITypstCompiler.CompileToPdf
                byte[] pdf = Typst.CompileToPdf("Hello from the C-sharp consumer");
                Require(pdf.Length > 4 && pdf[0] == (byte)'%' && pdf[1] == (byte)'P',
                    "compile yields a %PDF document");

                // CompileToPdf with sys.inputs
                byte[] pdfWithVars = Typst.CompileToPdf(
                    "#sys.inputs.heading",
                    new Dictionary<string, string> { ["heading"] = "Hi" });
                Require(pdfWithVars.Length > 4, "compile with vars yields a PDF");

                // Multi-format: one SVG buffer per page.
                byte[][] svgPages = Typst.Compile(
                    "#set page(width: 90pt, height: 60pt)\nSVG body",
                    NordocsFormat.Svg);
                Require(svgPages.Length == 1, "single-page source yields one SVG buffer");
                string svg0 = Encoding.UTF8.GetString(svgPages[0]);
                Require(svg0.StartsWith("<svg"), "the SVG buffer is an SVG document");

                // Source-map session as IDisposable.
                using (var session = Typst.CompileSession(
                    "#set page(width: 120pt, height: 80pt, margin: 10pt)\nJump"))
                {
                    Require(session.PageCount() == 1, "session has one page");
                    FfiPageSize size = session.PageSize(0);
                    Require(Math.Abs(size.width_pt - 120.0) < 0.01, "page width is 120pt");
                    string sessionSvg = session.Svg(0);
                    Require(sessionSvg.StartsWith("<svg"), "session SVG export works");
                    byte[] png = session.Png(0);
                    Require(png.Length > 8, "session PNG export works");
                    string jump = session.JumpFromClick(0, 115.0, 75.0);
                    Require(jump == "null", "empty-space click resolves to JSON null");
                }

                // Error path: a bad compile throws a typed NordocsException.
                bool threw = false;
                try
                {
                    Typst.CompileToPdf("#this_function_does_not_exist()");
                }
                catch (NordocsException ex)
                {
                    threw = true;
                    Require(ex.Code == FfiErrorCode.Compile, "compile failure surfaces FfiErrorCode.Compile");
                }

                Require(threw, "an invalid compile throws NordocsException");

                Console.WriteLine("nordocs C# consumer round-trip: OK");
                return 0;
            }
            catch (Exception ex)
            {
                Console.Error.WriteLine($"nordocs C# consumer round-trip FAILED: {ex}");
                return 1;
            }
        }

        private static void Require(bool condition, string what)
        {
            if (!condition)
            {
                throw new InvalidOperationException($"assertion failed: {what}");
            }
        }
    }
}
