<?php

namespace App\Model;

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
        $data_dir = getenv('DATA_DIR');
        return is_file($data_dir . '/' . $this->getId() . '/notice');
    }
}
