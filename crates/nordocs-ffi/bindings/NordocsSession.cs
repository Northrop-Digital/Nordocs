// Hand-written companion to the generated NordocsFfi.g.cs binding.
//
// This file is NOT generated — edit it freely. It gives the flat, C-ABI P/Invoke
// surface in NordocsFfi.g.cs an idiomatic C# face:
//
//   * NordocsException        — a thrown exception carrying the structured FfiError,
//                               so callers never inspect an out-parameter.
//   * Typst (static)          — the compile / markdown / preview surface, mirroring
//                               the reference ITypstCompiler / IMarkdownToTypstConverter
//                               / IPreviewRenderer (returns byte[] / string, throws on
//                               error), so swapping the implementation is mechanical.
//                               (Named `Typst`, not `Nordocs`, to avoid colliding with
//                               the `Nordocs` root namespace segment.)
//   * NordocsSession          — the opaque source-map session handle exposed as
//                               IDisposable (+ finalizer), so callers write
//                               `using var session = Typst.CompileSession(...)`,
//                               mirroring the reference's `using var compiler = ...`.
//
// All native-owned buffers/strings/errors are freed here exactly once; the session
// handle is nulled after Dispose so a double Dispose is a guarded no-op.

using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;

namespace Nordocs.Ffi
{
    /// <summary>The requested output format for <see cref="Typst.Compile"/>.</summary>
    public enum NordocsFormat
    {
        Pdf = 0,
        Svg = 1,
        Png = 2,
    }

    /// <summary>
    /// A failure surfaced from the native nordocs engine. <see cref="Code"/> mirrors
    /// the engine's typed error enum (the FFI <c>FfiErrorCode</c>).
    /// </summary>
    public sealed class NordocsException : Exception
    {
        public FfiErrorCode Code { get; }

        public NordocsException(FfiErrorCode code, string message)
            : base($"nordocs {code}: {message}")
        {
            Code = code;
        }
    }

    /// <summary>
    /// Marshalling helpers shared by the wrapper: turn the flat FFI value types back
    /// into managed <c>string</c>/<c>byte[]</c> and free the native allocation, and
    /// raise an exception when an out-parameter error is set.
    /// </summary>
    internal static class NordocsMarshal
    {
        /// <summary>Throw if <paramref name="err"/> reports a failure, freeing its message.</summary>
        internal static void ThrowOnError(FfiError err)
        {
            if (err.code == FfiErrorCode.Ok)
            {
                return;
            }

            // Read the message WITHOUT freeing it, then free the whole error once:
            // `ndoc_error_free` frees `err.message`, so freeing it here too would be
            // a double free.
            string message = ReadString(err.message);
            NordocsFfi.ndoc_error_free(err);
            throw new NordocsException(err.code, message);
        }

        /// <summary>Copy an owned <c>FfiString</c> to managed and free the native bytes.</summary>
        internal static string TakeString(FfiString s)
        {
            string managed = ReadString(s);
            NordocsFfi.ndoc_string_free(s);
            return managed;
        }

        /// <summary>Copy an owned <c>ByteBuffer</c> to managed and free the native bytes.</summary>
        internal static byte[] TakeBytes(ByteBuffer buffer)
        {
            byte[] managed = ReadBytes(buffer.data, buffer.len);
            NordocsFfi.ndoc_byte_buffer_free(buffer);
            return managed;
        }

        /// <summary>Read an <c>FfiString</c>'s bytes as UTF-8 without freeing it.</summary>
        internal static string ReadString(FfiString s)
        {
            if (s.data == IntPtr.Zero || s.len == 0)
            {
                return string.Empty;
            }

            byte[] bytes = ReadBytes(s.data, s.len);
            return Encoding.UTF8.GetString(bytes);
        }

        /// <summary>
        /// Copy a <c>ByteBuffer</c>'s bytes to managed WITHOUT freeing it — used for the
        /// inner buffers of a <c>CompileResult</c>, which are freed en masse by
        /// <c>ndoc_compile_result_free</c>.
        /// </summary>
        internal static byte[] CopyBytes(ByteBuffer buffer) => ReadBytes(buffer.data, buffer.len);

        private static byte[] ReadBytes(IntPtr data, ulong len)
        {
            if (data == IntPtr.Zero || len == 0)
            {
                return Array.Empty<byte>();
            }

            var managed = new byte[len];
            Marshal.Copy(data, managed, 0, (int)len);
            return managed;
        }
    }

    /// <summary>
    /// The compile / markdown / preview surface, mirroring the reference
    /// <c>ITypstCompiler</c>, <c>IMarkdownToTypstConverter</c>, and
    /// <c>IPreviewRenderer</c>. Each method returns a managed result and throws a
    /// <see cref="NordocsException"/> on failure (the FFI out-parameter is handled here).
    /// </summary>
    public static class Typst
    {
        /// <summary>
        /// Serialise a <c>sys.inputs</c> dictionary the way the FFI expects (a JSON
        /// object string); <c>null</c>/empty becomes an empty string (no inputs).
        /// </summary>
        private static string VarsJson(IReadOnlyDictionary<string, string>? variables)
        {
            if (variables == null || variables.Count == 0)
            {
                return string.Empty;
            }

            var sb = new StringBuilder("{");
            bool first = true;
            foreach (var kv in variables)
            {
                if (!first)
                {
                    sb.Append(',');
                }

                first = false;
                AppendJsonString(sb, kv.Key);
                sb.Append(':');
                AppendJsonString(sb, kv.Value);
            }

            sb.Append('}');
            return sb.ToString();
        }

        private static void AppendJsonString(StringBuilder sb, string value)
        {
            sb.Append('"');
            foreach (char c in value)
            {
                switch (c)
                {
                    case '"':
                        sb.Append("\\\"");
                        break;
                    case '\\':
                        sb.Append("\\\\");
                        break;
                    case '\n':
                        sb.Append("\\n");
                        break;
                    case '\r':
                        sb.Append("\\r");
                        break;
                    case '\t':
                        sb.Append("\\t");
                        break;
                    default:
                        sb.Append(c);
                        break;
                }
            }

            sb.Append('"');
        }

        /// <summary>Mirrors <c>ITypstCompiler.CompileToPdf</c>.</summary>
        public static byte[] CompileToPdf(string source, IReadOnlyDictionary<string, string>? variables = null)
        {
            ByteBuffer buffer = NordocsFfi.ndoc_compile_to_pdf(source, VarsJson(variables), out FfiError err);
            NordocsMarshal.ThrowOnError(err);
            return NordocsMarshal.TakeBytes(buffer);
        }

        /// <summary>Mirrors <c>ITypstCompiler.CompileFileToPdf</c>.</summary>
        public static byte[] CompileFileToPdf(string filePath)
        {
            ByteBuffer buffer = NordocsFfi.ndoc_compile_file_to_pdf(filePath, out FfiError err);
            NordocsMarshal.ThrowOnError(err);
            return NordocsMarshal.TakeBytes(buffer);
        }

        /// <summary>Mirrors <c>IMarkdownToTypstConverter.Convert</c>.</summary>
        public static string MarkdownToTypst(string markdown)
        {
            FfiString result = NordocsFfi.ndoc_markdown_to_typst(markdown, out FfiError err);
            NordocsMarshal.ThrowOnError(err);
            return NordocsMarshal.TakeString(result);
        }

        /// <summary>
        /// Multi-format compile. Returns one buffer per page for SVG/PNG, or a single
        /// buffer for PDF — matching <c>TypstSharp.CompileResult.Buffers[]</c>.
        /// </summary>
        public static byte[][] Compile(
            string source,
            NordocsFormat format,
            IReadOnlyDictionary<string, string>? variables = null,
            float dpi = 144.0f)
        {
            CompileResult result = NordocsFfi.ndoc_compile(
                source,
                VarsJson(variables),
                (FfiFormat)format,
                dpi,
                out FfiError err);
            NordocsMarshal.ThrowOnError(err);

            try
            {
                var pages = new byte[result.len][];
                int stride = Marshal.SizeOf<ByteBuffer>();
                for (int i = 0; i < (int)result.len; i++)
                {
                    var slot = new IntPtr(result.buffers.ToInt64() + (long)i * stride);
                    var buffer = Marshal.PtrToStructure<ByteBuffer>(slot);
                    pages[i] = NordocsMarshal.CopyBytes(buffer);
                }

                return pages;
            }
            finally
            {
                NordocsFfi.ndoc_compile_result_free(result);
            }
        }

        /// <summary>Mirrors <c>IPreviewRenderer.RenderComponentPreview</c> (inputs as JSON).</summary>
        public static byte[] RenderComponentPreview(
            string componentSource,
            string schemaJson,
            string? inputValuesJson = null,
            string? themeCode = null)
        {
            ByteBuffer buffer = NordocsFfi.ndoc_render_component_preview(
                componentSource,
                schemaJson,
                inputValuesJson ?? string.Empty,
                themeCode ?? string.Empty,
                out FfiError err);
            NordocsMarshal.ThrowOnError(err);
            return NordocsMarshal.TakeBytes(buffer);
        }

        /// <summary>Mirrors <c>IPreviewRenderer.RenderDocumentPreview</c> (inputs as JSON).</summary>
        public static byte[] RenderDocumentPreview(
            string documentStateJson,
            string themeCode,
            string componentSourcesJson,
            string componentSchemasJson)
        {
            ByteBuffer buffer = NordocsFfi.ndoc_render_document_preview(
                documentStateJson,
                themeCode,
                componentSourcesJson,
                componentSchemasJson,
                out FfiError err);
            NordocsMarshal.ThrowOnError(err);
            return NordocsMarshal.TakeBytes(buffer);
        }

        /// <summary>
        /// Compile a source into a reusable, disposable source-map session (multi-format
        /// export + click-to-source). Use with <c>using var session = ...</c>.
        /// </summary>
        public static NordocsSession CompileSession(
            string source,
            IReadOnlyDictionary<string, string>? variables = null)
        {
            IntPtr handle = NordocsFfi.ndoc_compile_session(source, VarsJson(variables), out FfiError err);
            NordocsMarshal.ThrowOnError(err);
            return new NordocsSession(handle);
        }
    }

    /// <summary>
    /// An opaque, retained compiled document the source map operates over, exposed as
    /// <see cref="IDisposable"/>. Mirrors the reference's <c>using var compiler = ...</c>:
    /// the native handle is released by <see cref="Dispose"/> (and a finalizer as a
    /// safety net), and nulled afterwards so a second Dispose is a no-op.
    /// </summary>
    public sealed class NordocsSession : IDisposable
    {
        private IntPtr _handle;

        internal NordocsSession(IntPtr handle)
        {
            _handle = handle;
        }

        private IntPtr Handle =>
            _handle != IntPtr.Zero
                ? _handle
                : throw new ObjectDisposedException(nameof(NordocsSession));

        /// <summary>Number of laid-out pages.</summary>
        public ulong PageCount()
        {
            ulong count = NordocsFfi.ndoc_session_page_count(Handle, out FfiError err);
            NordocsMarshal.ThrowOnError(err);
            return count;
        }

        /// <summary>Size of the page at 0-based <paramref name="index"/>, in points.</summary>
        public FfiPageSize PageSize(ulong index)
        {
            FfiPageSize size = NordocsFfi.ndoc_session_page_size(Handle, index, out FfiError err);
            NordocsMarshal.ThrowOnError(err);
            return size;
        }

        /// <summary>Export the 0-based <paramref name="page"/> as an SVG string.</summary>
        public string Svg(ulong page)
        {
            FfiString svg = NordocsFfi.ndoc_session_svg(Handle, page, out FfiError err);
            NordocsMarshal.ThrowOnError(err);
            return NordocsMarshal.TakeString(svg);
        }

        /// <summary>Render the 0-based <paramref name="page"/> as PNG bytes at <paramref name="dpi"/>.</summary>
        public byte[] Png(ulong page, float dpi = 144.0f)
        {
            ByteBuffer png = NordocsFfi.ndoc_session_png(Handle, page, dpi, out FfiError err);
            NordocsMarshal.ThrowOnError(err);
            return NordocsMarshal.TakeBytes(png);
        }

        /// <summary>
        /// Map a click on a rendered page back to its source location. Returns the
        /// JSON-serialised <c>Jump</c> (or the literal <c>null</c> when nothing resolves).
        /// </summary>
        public string JumpFromClick(ulong page, double xPt, double yPt)
        {
            FfiString jump = NordocsFfi.ndoc_session_jump_from_click(Handle, page, xPt, yPt, out FfiError err);
            NordocsMarshal.ThrowOnError(err);
            return NordocsMarshal.TakeString(jump);
        }

        /// <summary>
        /// Map a source cursor (a byte <paramref name="offset"/> into <paramref name="file"/>)
        /// to the on-page positions it produced. Returns a JSON array of <c>Position</c>s.
        /// </summary>
        public string JumpFromCursor(string file, ulong offset)
        {
            FfiString positions = NordocsFfi.ndoc_session_jump_from_cursor(Handle, file, offset, out FfiError err);
            NordocsMarshal.ThrowOnError(err);
            return NordocsMarshal.TakeString(positions);
        }

        public void Dispose()
        {
            ReleaseHandle();
            GC.SuppressFinalize(this);
        }

        ~NordocsSession()
        {
            ReleaseHandle();
        }

        private void ReleaseHandle()
        {
            // ndoc_session_free tolerates a null handle, but null our field first so a
            // double Dispose / Dispose-then-finalize never frees twice.
            IntPtr handle = _handle;
            _handle = IntPtr.Zero;
            if (handle != IntPtr.Zero)
            {
                NordocsFfi.ndoc_session_free(handle);
            }
        }
    }
}
