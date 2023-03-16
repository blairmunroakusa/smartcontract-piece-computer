(function() {var implementors = {
"uanft":[["impl ContractDispatchableConstructors&lt;/// - Main contract storage.\n    #[derive(Default, Storage)]\n    #[ink(storage)]\n    pub struct Psp34Nft {\n\n        /// - Openbrush PSP34 storage fields.\n        #[storage_field]\n        psp34: psp34::Data&lt;enumerable::Balances&gt;,\n\n        /// - Openbrush metadata extension storage fields.\n        #[storage_field]\n        metadata: metadata::Data,\n\n        /// - Openbrush ownable extension storage fields.\n        #[storage_field]\n        ownable: ownable::Data,\n\n        /// - Universal access NFT storage fields.\n        #[storage_field]\n        access: AccessData,\n\n        /// - Storage fields related to the UANFT as an application for the ILOCK PSP22 contract.\n        #[storage_field]\n        app: AppData,\n\n        /// - Art zero storage fields.\n        last_token_id: u64,\n        attribute_count: u32,\n        attribute_names: Mapping&lt;u32, Vec&lt;u8&gt;&gt;,\n        locked_tokens: Mapping&lt;Id, bool&gt;,\n        locked_token_count: u64,\n        is_attribute: Mapping&lt;String, bool&gt;,\n\n    }&gt; for <a class=\"struct\" href=\"uanft/uanft/struct.Psp34Nft.html\" title=\"struct uanft::uanft::Psp34Nft\">Psp34Nft</a>"]]
};if (window.register_implementors) {window.register_implementors(implementors);} else {window.pending_implementors = implementors;}})()