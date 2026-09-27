using System;
using System.Runtime.InteropServices;
using System.Runtime.InteropServices.ComTypes;

// MSI transforms are OLE substorages, not byte streams in the _Streams table.
public static class MsiStorage
{
    [ComImport, Guid("0000000B-0000-0000-C000-000000000046"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    private interface IStorage
    {
        void CreateStream([MarshalAs(UnmanagedType.LPWStr)] string name, uint mode, uint reserved1, uint reserved2, out IStream stream);
        void OpenStream([MarshalAs(UnmanagedType.LPWStr)] string name, IntPtr reserved1, uint mode, uint reserved2, out IStream stream);
        void CreateStorage([MarshalAs(UnmanagedType.LPWStr)] string name, uint mode, uint reserved1, uint reserved2, out IStorage storage);
        void OpenStorage([MarshalAs(UnmanagedType.LPWStr)] string name, IStorage priority, uint mode, IntPtr exclude, uint reserved, out IStorage storage);
        void CopyTo(uint excludeCount, IntPtr excludeIids, IntPtr excludeNames, IStorage destination);
        void MoveElementTo([MarshalAs(UnmanagedType.LPWStr)] string name, IStorage destination, [MarshalAs(UnmanagedType.LPWStr)] string newName, uint flags);
        void Commit(uint flags);
    }

    [DllImport("ole32.dll", CharSet = CharSet.Unicode, PreserveSig = false)]
    private static extern void StgOpenStorage(string path, IStorage priority, uint mode, IntPtr exclude, uint reserved, out IStorage storage);

    [DllImport("ole32.dll", CharSet = CharSet.Unicode, PreserveSig = false)]
    private static extern void StgCreateDocfile(string path, uint mode, uint reserved, out IStorage storage);

    public static void Extract(string package, string name, string destination)
    {
        IStorage root = null, child = null, output = null;
        try
        {
            // Read with deny-write sharing; create the output exclusively.
            StgOpenStorage(package, null, 0x20, IntPtr.Zero, 0, out root);
            root.OpenStorage(name, null, 0x10, IntPtr.Zero, 0, out child);
            StgCreateDocfile(destination, 0x1012, 0, out output);
            child.CopyTo(0, IntPtr.Zero, IntPtr.Zero, output);
            output.Commit(0);
        }
        finally
        {
            if (output != null) Marshal.FinalReleaseComObject(output);
            if (child != null) Marshal.FinalReleaseComObject(child);
            if (root != null) Marshal.FinalReleaseComObject(root);
        }
    }
}
