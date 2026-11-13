"""Packaged Markdown links keep their repository destinations after relocation."""
from pathlib import Path
from tempfile import TemporaryDirectory
import re
import unittest
from unittest.mock import patch

import build_installer as builder


class PackageDocsChecks(unittest.TestCase):
    def copy_document(self, source, target):
        # The previous builder copied Markdown verbatim; keep that as a baseline
        # so these checks also demonstrate the broken behavior before the fix.
        getattr(builder, 'copy_document', builder.copy)(source, target)

    def test_all_four_packaged_documents_have_no_repository_relative_links(self):
        sources=[builder.ROOT/'installer'/name for name in ('README-ES.md','README-EN.md','THIRD-PARTY.md')]
        sources.append(builder.ROOT/'CONTROLES.md')
        with TemporaryDirectory() as temp:
            for source in sources:
                original=source.read_bytes()
                target=Path(temp)/source.name
                self.copy_document(source,target)
                links=re.findall(r'\]\(([^)]+)\)',target.read_text(encoding='utf-8'))
                self.assertTrue(links)
                self.assertTrue(all('://' in link or link.startswith('#') for link in links),source.name)
                self.assertEqual(source.read_bytes(),original)
                self.assertIn('https://raw.githubusercontent.com/AdriiSuizo/jak-minecraft-java/main/docs/media/jak-worlds-header.gif',target.read_text(encoding='utf-8'))

    def test_spaces_fragments_queries_and_external_links(self):
        with TemporaryDirectory() as temp:
            root=Path(temp)/'repo';source=root/'installer/guide.md';source.parent.mkdir(parents=True)
            (root/'docs').mkdir()
            (root/'docs/read me.md').write_text('guide',encoding='utf-8')
            (root/'docs/some art.gif').write_bytes(b'fixture')
            text='\n'.join([
                '[guide](../docs/read%20me.md?plain=1#my-heading "title")',
                '![art](<../docs/some art.gif#preview> "image title")',
                '[here](#local-heading)',
                '[external](https://example.com/a%20b?x=1#y)',
                '[mail](mailto:example@example.com)',
            ])
            source.write_text(text,encoding='utf-8');target=Path(temp)/'package/guide.md'
            with patch.object(builder,'ROOT',root):self.copy_document(source,target)
            result=target.read_text(encoding='utf-8')
            self.assertIn('https://github.com/AdriiSuizo/jak-minecraft-java/blob/main/docs/read%20me.md?plain=1#my-heading "title"',result)
            self.assertIn('<https://raw.githubusercontent.com/AdriiSuizo/jak-minecraft-java/main/docs/some%20art.gif#preview> "image title"',result)
            for line in text.splitlines()[2:]:self.assertIn(line,result)
            self.assertEqual(source.read_text(encoding='utf-8'),text)


if __name__=='__main__':unittest.main()
