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
        public const string BundleId = "{60314D27-47DF-4118-B937-6D1445BAC9D7}";

        public override void SymbolsFinalized(IntermediateSection section)
        {
            base.SymbolsFinalized(section);

            var bundle = section.Symbols.OfType<WixBundleSymbol>().Single();
            bundle.BundleId = BundleId;
        }
    }
}
