using System;
using System.Collections.Generic;
using System.Linq;
using WixToolset.Data;
using WixToolset.Data.Symbols;
using WixToolset.Extensibility;

namespace Gnx.DeterministicBundle
{
    public sealed class DeterministicBundleExtensionFactory : BaseExtensionFactory
    {
        protected override IReadOnlyCollection<Type> ExtensionTypes =>
            new[] { typeof(DeterministicBundleBackendExtension) };
    }

    public sealed class DeterministicBundleBackendExtension : BaseBurnBackendBinderExtension
    {
        public const string BundleId = "{A3A9B194-9B26-4C85-A240-AAA053B8D433}";

        public override void SymbolsFinalized(IntermediateSection section)
        {
            base.SymbolsFinalized(section);

            var bundle = section.Symbols.OfType<WixBundleSymbol>().Single();
            bundle.BundleId = BundleId;
        }
    }
}
