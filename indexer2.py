#!/usr/bin/env python
# -*- coding: utf-8 -*-
"""
Copyright (C) 2012 Legoktm

Permission is hereby granted, free of charge, to any person obtaining
a copy of this software and associated documentation files (the "Software"),
to deal in the Software without restriction, including without limitation
the rights to use, copy, modify, merge, publish, distribute, sublicense,
and/or sell copies of the Software, and to permit persons to whom the
Software is furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in
all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS
OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS
IN THE SOFTWARE.
"""
import pywikibot
from pywikibot.pagegenerators import ReferringPageGenerator
import robot
import index_help

"""
(C) Legoktm, 2012 under the MIT License

"""

# constants
MONTH_NAMES = ('January', 'February', 'March', 'April', 'May', 'June', 'July', 'August', 'September', 'October',
               'November', 'December')
MONTH_REGEX = '|'.join(month for month in MONTH_NAMES)


class IndexBot(robot.Robot):

    def __init__(self):
        robot.Robot.__init__(self, task=15)
        self.template = pywikibot.Page(self.site, 'User:HBC Archive Indexerbot/OptIn')
        self.start_logging(pywikibot.Page(self.site, 'User:Legobot/Archive Indexer Log'))

    def process_pages(self):
        pages = []
        for arg in self.args:
            if arg.startswith('--page'):
                pages = [pywikibot.Page(self.site, arg[7:])]

        if not pages:
            print('No pages provided, processing all pages...')
            pages = ReferringPageGenerator(self.template, onlyTemplateInclusion=True, content=True)
        for page in pages:
            print('Processing %s' % page.title())
            self.do_page(page)

    def do_page(self, page):
        info = index_help.parse_instructions(page)
        text = index_help.follow_instructions(info)
        if text:
            self.output(text)


def main():
    bot = IndexBot()
    try:
        bot.process_pages()
    finally:
        bot.log_text = 'Run finished at ~~~~~\n' + bot.log_text
        # bot.pushLog(overwrite=True)


if __name__ == "__main__":
    main()
