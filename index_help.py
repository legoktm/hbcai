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


"""
helper functions for indexer2.py
"""
import re
import urllib.parse
import difflib
import time
import calendar
import datetime
import pywikibot
from pywikibot.textlib import removeDisabledParts

SITE = pywikibot.Site()
LOG_TEXT = ''
MONTH_NAMES = ('January', 'February', 'March', 'April', 'May', 'June', 'July', 'August', 'September', 'October',
               'November', 'December')
MONTH_REGEX = '|'.join(month for month in MONTH_NAMES)


def parse_instructions(page):
    """
    Parses the index template for all of the parameters
    """
    text = page.get()
    # print u'Parsing instructions for [[%s]].' % page.title()
    key = text.find('{{User:HBC Archive Indexerbot/OptIn')
    data = text[key:].split('}}')[0][36:]  # kinda scared about hardcoding so much
    # remove any comments (apparently users do this)
    cleaned = removeDisabledParts(data)
    info = {}
    info['mask'] = []
    info['talkpage'] = page.title()
    for param in cleaned.split('|'):
        param = clean(param)
        if param.startswith('target='):
            target = clean(param[7:])
            if target.startswith('/'):
                target = page.title() + target
            elif target.startswith('./'):
                target = page.title() + target[1:]
            info['target'] = target
        elif param.startswith('mask='):
            mask = clean(param[5:])
            if mask.startswith('/'):
                mask = page.title() + mask
            elif mask.startswith('./'):
                mask = page.title() + mask[1:]
            info['mask'].append(mask)
        elif param.startswith('indexhere='):
            value = param[10:]
            if clean(value.lower()) == 'yes':
                info['indexhere'] = True
            else:
                info['indexhere'] = False
        elif param.startswith('template='):
            info['template'] = clean(param[9:].replace('\n', ''))
        elif param.startswith('leading_zeros='):
            try:
                info['leading_zeros'] = int(clean(param[14:]))
            except ValueError:
                pass
        elif param.startswith('first_archive='):
            info['first_archive'] = clean(param[14:])
    # set default values if not already set
    for key in list(info):
        if isinstance(info[key], str):
            if info[key].isspace() or (not info[key]):
                del info[key]

    if 'leading_zeros' not in info:
        info['leading_zeros'] = 0
    if 'indexhere' not in info:
        info['indexhere'] = False
    if 'template' not in info:
        info['template'] = 'User:HBC Archive Indexerbot/default template'
    if info['template'] == 'template location':
        info['template'] = 'User:HBC Archive Indexerbot/default template'
    return info


def clean(text):
    """various cleaning functions to simplify parsing bad text"""

    # first lets eliminate any useless whitespace
    text = text.strip()
    # clean up when people do |indehere=<yes>
    search = re.search('(.*?)=\<(#|yes|no|month|year|.*?)\>', text)
    if search:
        front = search.group(1) + '='
        if search.group(2) in ['#', 'month', 'year']:
            pass
        elif search.group(2) in ['yes', 'no']:
            text = search.group(2)
        else:
            text = search.group(2)
        text = front + text
    # remove wikilinks from everything
    search = re.search('\[\[(.*?)\]\]', text)
    if search:
        text = text.replace(search.group(0), search.group(1))
    return text


def prefix_number(num, leading):
    """
    Prefixes "num" with %leading zeroes.
    """
    length = int(leading) + 1
    num = str(num)
    while len(num) < length:
        num = '0' + num
    return num


def next_month(month, year):
    """
    Returns what the next month should be
    If December --> January, then it ups the year as well
    """

    index = MONTH_NAMES.index(month)
    if index == 11:
        new_index = 0
        year += 1
    else:
        new_index = index + 1
    new_month = MONTH_NAMES[new_index]
    return new_month, year


def get_next_mask(current, pattern, leading_zeroes=0):
    if '<#>' in pattern:
        regex = pattern.replace('<#>', '(\d+)')
        key = int(re.search(regex, current).group(1))
        archive_num = prefix_number(key+1, leading_zeroes)
        return pattern.replace('<#>', archive_num)
    if '<month>' in pattern:
        regex = pattern.replace('<month>', '(%s)' % MONTH_REGEX).replace('<year>', '(\d\d\d\d)')
        match = re.search(regex, current)
        month, year = next_month(match.group(1), int(match.group(2)))
        return pattern.replace('<month>', month).replace('<year>', str(year))


def get_masks(info):
    data = list()
    for mask in info['mask']:
        if '<#>' in mask:
            key = 1
            keep_going = True
            # numerical archive
            while keep_going:
                archive_num = prefix_number(key, info['leading_zeros'])
                title = mask.replace('<#>', archive_num)
                page = pywikibot.Page(SITE, title)
                key += 1
                if page.exists():
                    data.append(page)
                else:
                    keep_going = False

        elif '<month>' in mask:
            if 'first_archive' not in info:
                raise Exception('No mask found')
            # grab the month and year out of the first archive
            regex = mask.replace('<month>', '(%s)' % MONTH_REGEX).replace('<year>', '(\d\d\d\d)')
            match = re.search(regex, info['first_archive'])
            month = match.group(1)
            year = int(match.group(2))
            keep_going = True
            while keep_going:
                title = mask.replace('<month>', month).replace('<year>', str(year))
                page = pywikibot.Page(SITE, title)
                if page.exists():
                    data.append(page)
                    month, year = next_month(month, year)
                else:
                    keep_going = False
        elif '<year>' in mask:  # special case for when only a year is provided
            regex = mask.replace('<year>', '(\d\d\d\d)')
            match = re.search(regex, info['first_archive'])
            year = int(match.group(1))
            keep_going = True
            while keep_going:
                title = mask.replace('<year>', str(year))
                page = pywikibot.Page(SITE, title)
                if page.exists():
                    data.append(page)
                    year += 1
                else:
                    keep_going = False
        else:  # assume the mask is the page
            if ('<' in mask) or ('>' in mask):
                print('ERRORERROR: Did not parse %s properly.' % mask)
                continue
            page = pywikibot.Page(SITE, mask)
            if page.exists():
                data.append(page)
    if info['indexhere']:
        data.append(pywikibot.Page(SITE, info['talkpage']))
    return data


def follow_instructions(info):
    # verify all required parameters are there
    if 'mask' not in info or 'target' not in info:
        return '* [[:%s]] has an incorrectly configured template.' % info['talkpage']
    # verify we can edit the target, otherwise just skip it
    # hopefully this will save processing time
    indexPage = pywikibot.Page(SITE, info['target'])
    talkPage = pywikibot.Page(SITE, info['talkpage'])
    try:
        indexPageOldText = indexPage.get()
    except pywikibot.exceptions.IsRedirectPage:
        indexPage = indexPage.getRedirectTarget()
        indexPageOldText = indexPage.get()
    except pywikibot.exceptions.NoPage:
        return '* [[:%s]] does not have the safe string.' % info['talkpage']
    if not ok_to_edit(indexPageOldText):
        return '* [[:%s]] does not have the safe string.' % info['talkpage']
    edittime = pywikibot.Timestamp.fromISOformat(indexPage.editTime())
    twelvehr = datetime.datetime.utcnow() - datetime.timedelta(hours=12)
    if twelvehr < edittime:
        print('Edited %s less than 12 hours ago.' % indexPage.title())
        # return
    # looks good, lets go
    data = {}
    # first process the mask
    masks = get_masks(info)
    data['archives'] = masks
    # finished the mask processing!
    # now verify the template exists
    template = pywikibot.Page(SITE, info['template'])
    if not template.exists():
        # fallback on the default template
        template = pywikibot.Page(SITE, 'User:HBC Archive Indexerbot/default template')
    data['template'] = template.get()
    # finished the template part
    # lets parse all of the archives now
    data['parsed'] = list()
    for page in SITE.preloadpages(data['archives']):
        parsed = parse_archive(page)
        data['parsed'].extend(parsed)
    # build the index
    indexText = build_index(data['parsed'], data['template'], info)
    print('Will edit %s' % indexPage.title())
    # pywikibot.showDiff(indexPageOldText, indexText)
    if verify_update(indexPageOldText, indexText):
        indexPage.put(indexText, 'BOT: Updating index', async=True)
        return '* Successfully indexed [[%s]] to [[%s]].\n' % (talkPage.title(), indexPage.title())
    else:
        return '* [[%s]] did not require a new update.\n' % talkPage.title()


def ok_to_edit(text):
    return bool(re.search('<!-- (HBC Archive Indexerbot|Legobot) can blank this -->', text))


def build_index(parsedData, template, info):
    """
    Reads the template and creates the index for it
    """
    # first lets read the template
    # print('Building the index.')
    template_data = {}
    key = template.find('<nowiki>')
    last_key = template.find('</nowiki>')
    if key == -1:
        key = template.find('<pre>')
        last_key = template.find('</pre>')
    important_stuff = template[key+8:last_key]
    split = re.split('<!--\s', important_stuff)
    for item in split:
        if item.startswith('HEADER'):
            template_data['header'] = item[11:]
        elif item.startswith('ROW'):
            template_data['row'] = item[8:]
        elif item.startswith('ALT ROW'):
            template_data['altrow'] = item[12:]
        elif item.startswith('FOOTER'):
            template_data['footer'] = item[11:]
        elif item.startswith('END'):
            template_data['end'] = item[8:]
        elif item.startswith('LEAD'):
            template_data['lead'] = item[9:]
    if 'altrow' not in template_data:
        template_data['altrow'] = template_data['row']
    if 'lead' not in template_data:
        template_data['lead'] = ''
    if 'end' not in template_data:
        template_data['end'] = ''
    # print(template_data)
    # finished reading the template
    index_text = '<!-- HBC Archive Indexerbot can blank this -->'
    index_text += template_data['lead']
    report_info = 'Report generated based on a request from [[%s]]. It matches the following masks: ' % \
                  pywikibot.Page(SITE, info['talkpage']).title()
    report_info += ' ,'.join([m.strip() for m in info['mask']])
    report_info += '\n<br />\nIt was generated at ~~~~~ by [[User:Legobot|Legobot]].\n'
    index_text += report_info
    index_text += template_data['header']
    alt = False
    for item in parsedData:
        if alt:
            row_text = template_data['altrow']
            alt = False
        else:
            row_text = template_data['row']
            alt = True
        row_text = row_text.replace('%%topic%%', item['topic'])
        row_text = row_text.replace('%%replies%%', str(item['replies']))
        row_text = row_text.replace('%%link%%', item['link'])
        row_text = row_text.replace('%%first%%', item['first'])
        row_text = row_text.replace('%%firstepoch%%', str(item['firstepoch']))
        row_text = row_text.replace('%%last%%', item['last'])
        row_text = row_text.replace('%%lastepoch%%', str(item['lastepoch']))
        row_text = row_text.replace('%%duration%%', item['duration'])
        row_text = row_text.replace('%%durationsecs%%', str(item['durationsecs']))
        index_text += row_text
    index_text += template_data['footer']
    index_text += template_data['end']
    return index_text


def parse_archive(page):
    """
    Parses each individual archive
    Returns a list of dicts of the following info:
        topic - The heading
        replies - estimated count (simply finds how many instances of "(UTC)" are present
        link - link to that section
        first - first comment
        firstepoch - first comment (epoch)
        last - last comment
        lastepoch - last comment (epoch)
        duration - last-first (human readable)
        durationsecs - last-first (seconds)

    """
    tmp_page = page
    while tmp_page.isRedirectPage():
        tmp_page = tmp_page.getRedirectTarget()
    text = tmp_page.get()
    print('Parsing %s.' % page.title())
    threads = split_into_threads(text)
    data = list()
    for thread in threads:
        d = {}
        d['topic'] = thread['topic'].strip()
        d['link'] = '[[%s#%s]]' % (page.title(), clean_links(d['topic']))
        content = thread['content']
        d['content'] = content
        # hackish way of finding replies
        found = re.findall('\(UTC\)', content)
        d['replies'] = len(found)
        # find all the timestamps
        ts = re.finditer('(\d\d:\d\d|\d\d:\d\d:\d\d), (\d\d) (%s) (\d\d\d\d)' % MONTH_REGEX, content)
        epochs = list()
        for stamp in ts:
            mw = stamp.group(0)
            parsed = mw_to_epoch(mw)
            if parsed:
                epochs.append(calendar.timegm(parsed))
        earliest = 999999999999999999
        last = 0
        for item in epochs:
            if item < earliest:
                earliest = item
            if item > last:
                last = item
        if earliest == 999999999999999999:
            earliest = 'Unknown'
            d['duration'] = 'Unknown'
            d['durationsecs'] = 'Unknown'
        if last == 0:
            last = 'Unknown'
            d['duration'] = 'Unknown'
            d['durationsecs'] = 'Unknown'

        d['first'] = epoch_to_mw(earliest)
        d['firstepoch'] = earliest
        d['last'] = epoch_to_mw(last)
        d['lastepoch'] = last
        if 'duration' not in d:
            d['duration'] = human_readable(last - earliest)
            d['durationsecs'] = last - earliest
        data.append(d)
    return data


def split_into_threads(text, level3=False):
    """
    Inspired/Copied by/from pywikipedia/archivebot.py
    """
    if level3:
        regex = '^=== *([^=].*?) *=== *$'
    else:
        regex = '^== *([^=].*?) *== *$'
    lines = text.split('\n')
    found = False
    threads = list()
    current_thread = {}
    for line in lines:
        thread_header = re.search(regex, line)
        if thread_header:
            found = True
            if current_thread:
                threads.append(current_thread)
                current_thread = {}
            current_thread['topic'] = thread_header.group(1)
            current_thread['content'] = ''
        else:
            if found:
                current_thread['content'] += line + '\n'
    if current_thread:
        threads.append(current_thread)
    if not threads and not level3:
        threads = split_into_threads(text, level3=True)
    return threads


def clean_links(link):
    # [[piped|links]] --> links
    search = re.search('\[\[:?(.*?)\|(.*?)\]\]', link)
    while search:
        link = link.replace(search.group(0), search.group(2))
        search = re.search('\[\[:?(.*?)\|(.*?)\]\]', link)
    # [[wikilinks]] --> wikilinks
    search = re.search('\[\[:?(.*?)\]\]', link)
    while search:
        link = link.replace(search.group(0), search.group(1))
        search = re.search('\[\[:?(.*?)\]\]', link)
    # '''bold''' --> bold
    # ''italics'' --> italics
    search = re.search("('''|'')(.*?)('''|'')", link)
    while search:
        link = link.replace(search.group(0), search.group(2))
        search = re.search("('''|'')(.*?)('''|'')", link)
    # <nowiki>blah</nowiki> --> blah
    link = link.replace('<nowiki>', '').replace('</nowiki>', '')
    link = urllib.parse.quote(link)
    return link


def epoch_to_mw(timestamp):
    """
    Converts a unix epoch time to a mediawiki timestamp
    """
    if isinstance(timestamp, str):
        return timestamp
    struct = time.gmtime(timestamp)
    return time.strftime('%H:%M, %d %B %Y', struct)


def mw_to_epoch(timestamp):
    """
    Converts a mediawiki timestamp to unix epoch time
    """
    try:
        return time.strptime(timestamp, '%H:%M, %d %B %Y')
    except ValueError:
        try:
            return time.strptime(timestamp, '%H:%M:%S, %d %B %Y')
            # Some users (ex: Pathoschild) include seconds in their signature
        except ValueError:
            return None  # srsly wtf?


def human_readable(seconds):
    return str(datetime.timedelta(seconds=seconds))


def verify_update(old, new):
    """
    Verifies than an update is needed, and we won't be just updating the timestamp
    """
    old2 = re.sub('generated at (.*?) by', 'generated at ~~~~~ by', old)
    # pywikibot.showDiff(old2, new)
    update = False
    for line in difflib.ndiff(old2.splitlines(), new.splitlines()):
        if not line.startswith(' '):
            if line.startswith('+'):
                if not line[1:].isspace():
                    update = True
                    break
            elif line.startswith('-'):
                if not line[1:].isspace():
                    update = True
                    break
    return update
