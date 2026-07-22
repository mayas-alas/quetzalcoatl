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
        public const string BundleId = "{6FC46C58-8F5B-44E8-90D4-9E5E90A3EC33}";

        public override void SymbolsFinalized(IntermediateSection section)
        {
            base.SymbolsFinalized(section);

            var bundle = section.Symbols.OfType<WixBundleSymbol>().Single();
            bundle.BundleId = BundleId;
        }
    }
}
