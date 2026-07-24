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
        public const string BundleId = "{42D4F602-1355-5B82-B60C-2E5D7F03BFB5}";

        public override void SymbolsFinalized(IntermediateSection section)
        {
            base.SymbolsFinalized(section);

            var bundle = section.Symbols.OfType<WixBundleSymbol>().Single();
            bundle.BundleId = BundleId;
        }
    }
}
