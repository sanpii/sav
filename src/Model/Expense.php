<?php

namespace Model;

use PommProject\ModelManager\Model\FlexibleEntity;

/**
 * Expense
 *
 * Flexible entity for relation
 * public.expense
 *
 * @see FlexibleEntity
 */
class Expense extends FlexibleEntity
{
    public function hasNotice()
    {
        return is_file(__DIR__ . '/../../data/' . $this->getId() . '/notice');
    }
}
